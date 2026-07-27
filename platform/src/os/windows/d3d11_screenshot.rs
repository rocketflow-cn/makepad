use crate::{
    cx::Cx,
    os::windows::d3d11::{D3d11Cx, D3d11Window},
    windows::{
        core::Interface,
        Win32::Graphics::{
            Direct3D11::{
                D3D11_CPU_ACCESS_FLAG, D3D11_MAP, D3D11_MAPPED_SUBRESOURCE,
                D3D11_TEXTURE2D_DESC, D3D11_USAGE, ID3D11DeviceContext, ID3D11Resource,
                ID3D11Texture2D,
            },
            Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
        },
    },
};

const D3D11_CPU_ACCESS_READ_LOCAL: D3D11_CPU_ACCESS_FLAG = D3D11_CPU_ACCESS_FLAG(0x20_000);
const D3D11_MAP_READ_LOCAL: D3D11_MAP = D3D11_MAP(1);
const D3D11_USAGE_STAGING_LOCAL: D3D11_USAGE = D3D11_USAGE(3);
const BYTES_PER_PIXEL: usize = 4;

pub(super) fn capture_requested_screenshot(
    cx: &mut Cx,
    window: &D3d11Window,
    d3d11_cx: &D3d11Cx,
) {
    let Some(source_texture) = window.swap_texture.as_ref() else {
        return;
    };
    let request_ids = cx.take_studio_screenshot_request_ids(window.window_id.id() as u32);
    if request_ids.is_empty() {
        return;
    }

    let result = readback_swap_texture(window, d3d11_cx, source_texture).and_then(
        |(width, height, rgba)| {
            Cx::encode_rgba_as_png(width, height, &rgba)
                .map(|png| (width, height, png))
        },
    );
    match result {
        Ok((width, height, png)) => {
            Cx::send_studio_screenshot_response(request_ids, width, height, png);
        }
        Err(error) => {
            crate::error!("D3D11 screenshot readback failed: {error}");
        }
    }
}

fn readback_swap_texture(
    window: &D3d11Window,
    d3d11_cx: &D3d11Cx,
    source_texture: &ID3D11Texture2D,
) -> Result<(u32, u32, Vec<u8>), String> {
    let width = physical_dimension(window.window_geom.inner_size.x, window.window_geom.dpi_factor)?;
    let height = physical_dimension(window.window_geom.inner_size.y, window.window_geom.dpi_factor)?;
    let staging_texture = create_staging_texture(d3d11_cx, width, height)?;
    let source_resource: ID3D11Resource = source_texture
        .cast()
        .map_err(|error| format!("cast swap texture to resource failed: {error}"))?;
    let staging_resource: ID3D11Resource = staging_texture
        .cast()
        .map_err(|error| format!("cast staging texture to resource failed: {error}"))?;

    copy_resource(&d3d11_cx.context, &staging_resource, &source_resource);
    let mapped = map_read(&d3d11_cx.context, &staging_resource)?;
    let rgba = copy_bgra_rows(&mapped, width, height)?;
    Ok((width, height, rgba))
}

fn physical_dimension(logical: f64, dpi_factor: f64) -> Result<u32, String> {
    let physical = logical * dpi_factor;
    if !physical.is_finite() || physical < 1.0 || physical > u32::MAX as f64 {
        return Err(format!("invalid swap texture dimension: {physical}"));
    }
    Ok(physical as u32)
}

fn create_staging_texture(
    d3d11_cx: &D3d11Cx,
    width: u32,
    height: u32,
) -> Result<ID3D11Texture2D, String> {
    let descriptor = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_STAGING_LOCAL,
        BindFlags: 0,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ_LOCAL.0 as u32,
        MiscFlags: 0,
    };
    let mut staging_texture = None;
    // SAFETY: `descriptor` and the output slot remain valid for the duration of the call.
    // No initial data is supplied, and the returned COM object is owned by its RAII wrapper.
    unsafe {
        d3d11_cx
            .device
            .CreateTexture2D(&descriptor, None, Some(&mut staging_texture))
            .map_err(|error| format!("create staging texture failed: {error}"))?;
    }
    staging_texture.ok_or_else(|| "CreateTexture2D returned no staging texture".to_string())
}

fn copy_resource(
    context: &ID3D11DeviceContext,
    destination: &ID3D11Resource,
    source: &ID3D11Resource,
) {
    // SAFETY: Both COM resources are valid and owned for the duration of the call. They have
    // identical dimensions and BGRA8 layouts, and D3D11 serializes the copy on this immediate
    // context before a subsequent blocking Map.
    unsafe {
        let vtable = Interface::vtable(context);
        (vtable.CopyResource)(
            Interface::as_raw(context),
            Interface::as_raw(destination),
            Interface::as_raw(source),
        );
    }
}

fn map_read<'a>(
    context: &'a ID3D11DeviceContext,
    resource: &'a ID3D11Resource,
) -> Result<MappedTexture<'a>, String> {
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    // SAFETY: `resource` was created as a CPU-readable staging texture. `mapped` remains valid
    // for the call, and `MappedTexture::drop` pairs every successful Map with exactly one Unmap.
    unsafe {
        context
            .Map(resource, 0, D3D11_MAP_READ_LOCAL, 0, Some(&mut mapped))
            .map_err(|error| format!("map staging texture failed: {error}"))?;
    }
    if mapped.pData.is_null() {
        // SAFETY: Map succeeded for subresource 0, so it must be paired with Unmap even when the
        // driver unexpectedly returns a null data pointer.
        unsafe {
            context.Unmap(resource, 0);
        }
        return Err("mapped staging texture has a null data pointer".to_string());
    }
    Ok(MappedTexture {
        context,
        resource,
        data: mapped.pData.cast(),
        row_pitch: mapped.RowPitch as usize,
    })
}

fn copy_bgra_rows(
    mapped: &MappedTexture<'_>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, String> {
    let row_bytes = (width as usize)
        .checked_mul(BYTES_PER_PIXEL)
        .ok_or_else(|| "screenshot row byte count overflowed".to_string())?;
    if mapped.row_pitch < row_bytes {
        return Err(format!(
            "mapped row pitch {} is smaller than row size {row_bytes}",
            mapped.row_pitch
        ));
    }
    let height = height as usize;
    let last_row_offset = mapped
        .row_pitch
        .checked_mul(height.saturating_sub(1))
        .ok_or_else(|| "mapped screenshot row offset overflowed".to_string())?;
    let mapped_len = last_row_offset
        .checked_add(row_bytes)
        .filter(|length| *length <= isize::MAX as usize)
        .ok_or_else(|| "mapped screenshot byte count overflowed".to_string())?;
    let output_len = row_bytes
        .checked_mul(height)
        .ok_or_else(|| "screenshot output byte count overflowed".to_string())?;
    // SAFETY: A successful D3D11 Map exposes each row at a `RowPitch` offset. The checked length
    // ends at the last row's final pixel, is within `isize::MAX`, and remains valid until the
    // `mapped` guard drops.
    let source = unsafe { std::slice::from_raw_parts(mapped.data, mapped_len) };
    let mut rgba = vec![0; output_len];
    for row in 0..height {
        let source_start = row * mapped.row_pitch;
        let output_start = row * row_bytes;
        let source_row = &source[source_start..source_start + row_bytes];
        let output_row = &mut rgba[output_start..output_start + row_bytes];
        for (bgra, rgba_pixel) in source_row
            .chunks_exact(BYTES_PER_PIXEL)
            .zip(output_row.chunks_exact_mut(BYTES_PER_PIXEL))
        {
            rgba_pixel.copy_from_slice(&[bgra[2], bgra[1], bgra[0], bgra[3]]);
        }
    }
    Ok(rgba)
}

struct MappedTexture<'a> {
    context: &'a ID3D11DeviceContext,
    resource: &'a ID3D11Resource,
    data: *const u8,
    row_pitch: usize,
}

impl Drop for MappedTexture<'_> {
    fn drop(&mut self) {
        // SAFETY: This guard is constructed only after Map succeeds and is its unique owner, so
        // dropping it pairs that Map with exactly one Unmap of the same resource/subresource.
        unsafe {
            self.context.Unmap(self.resource, 0);
        }
    }
}

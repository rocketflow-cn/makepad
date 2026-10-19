//! One-shot application screenshots using the platform capture backends.
use std::{collections::HashMap, sync::{Mutex, OnceLock}};
use makepad_studio_protocol::ScreenshotResponse;
use crate::{cx::Cx, screen_capture::{add_screen_capture, remove_screen_capture, ScreenCaptureOptions}, thread::SignalToUI, window::WindowId};

fn responses() -> &'static Mutex<HashMap<u64, Option<ScreenshotResponse>>> {
    static RESPONSES: OnceLock<Mutex<HashMap<u64, Option<ScreenshotResponse>>>> = OnceLock::new();
    RESPONSES.get_or_init(|| Mutex::new(HashMap::new()))
}

impl Cx {
    /// Request the next presented frame from any application window.
    pub fn request_screenshot(&mut self) -> u64 {
        self.request_application_screenshot(None)
    }

    /// Request the next presented frame from a specific application window.
    pub fn request_window_screenshot(&mut self, window_id: WindowId) -> u64 {
        self.request_application_screenshot(Some(window_id))
    }

    fn request_application_screenshot(&mut self, window_id: Option<WindowId>) -> u64 {
        use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
        let capture_id = Arc::new(AtomicU64::new(0));
        let callback_id = capture_id.clone();
        let mut captured = false;
        let id = add_screen_capture(ScreenCaptureOptions {
            window_id: window_id.map(|id| id.0),
            max_fps: 0.0,
        }, move |frame| {
            if captured { return; }
            let id = callback_id.load(Ordering::Acquire);
            if id == 0 { return; }
            match Cx::encode_rgba_as_png(frame.width, frame.height, frame.rgba) {
                Ok(png) => {
                    responses().lock().unwrap().insert(id, Some(ScreenshotResponse {
                        request_ids: vec![id], png, width: frame.width, height: frame.height,
                    }));
                    captured = true;
                    SignalToUI::set_ui_signal();
                }
                Err(error) => crate::error!("Application screenshot encoding failed: {}", error),
            }
        });
        responses().lock().unwrap().insert(id, None);
        capture_id.store(id, Ordering::Release);
        self.redraw_all();
        id
    }

    /// Take a completed capture. Call after receiving a UI signal.
    pub fn take_screenshot_response(&mut self, request_id: u64) -> Option<ScreenshotResponse> {
        let response = {
            let mut pending = responses().lock().unwrap();
            if !pending.get(&request_id).is_some_and(Option::is_some) { return None; }
            pending.remove(&request_id).flatten()
        };
        remove_screen_capture(request_id);
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen_capture::deliver_capture_frame;

    #[test]
    fn application_capture_filters_window_and_returns_only_one_frame() {
        let mut cx = Cx::new(Box::new(|_, _| {}));
        let request = cx.request_window_screenshot(WindowId(123456, 0));
        deliver_capture_frame(Some(123455), 1, 1, &[255, 0, 0, 255]);
        assert!(cx.take_screenshot_response(request).is_none());
        deliver_capture_frame(Some(123456), 1, 1, &[0, 255, 0, 255]);
        deliver_capture_frame(Some(123456), 2, 1, &[0; 8]);
        let result = cx.take_screenshot_response(request).unwrap();
        assert_eq!(result.request_ids, vec![request]);
        assert_eq!((result.width, result.height), (1, 1));
        assert!(!result.png.is_empty());
        assert!(cx.take_screenshot_response(request).is_none());
    }
}

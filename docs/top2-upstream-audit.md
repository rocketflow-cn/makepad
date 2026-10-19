# top2 upstream migration audit

Baseline: official origin/dev `71c9ddcbb9c01dd208f99621fdd71024a5d35c65`.
Original fork preserved at `backup/topfi-before-official-20261019` (`18e0bfa63`).
The histories are unrelated; top2 starts at official dev and reapplies functionality.
Scope: widgets and dependencies required by widget consumers, including DeltaPex.

| Original commit | Decision | Result / evidence |
| --- | --- | --- |
| fdd13a93a | adapt | Preserve first initialization draw and focus before main draw list exists. Official `compute_pass_repaint_order` already clears orphaned pass dirtiness through attachment liveness. |
| 30f58c802 | keep | Reset animator on script reload; migrated to widgets/core. |
| 500af0677 | adapt | Keep native chrome positioning API; official caption centering is a different feature. |
| 05a0ce4d0 | adapt | Keep Pal premultiplied mixing; adapt radio and new button face/stroke implementations. |
| 073ad44af | drop | Official cursor dispatch tests respondsToSelector, checks nil, and falls back to arrow. |
| fa0d68d76 | adapt | Keep web composition guard and TextInput composition submit guards; preserve official MacosImeKeyboard transaction handling. |
| 40d6db4d2 | keep | Windows IME composition placement patch applies to current backend. |
| cb3f9e81b | drop | Official first_rect_for_character_range already converts ime_rect into AppKit screen coordinates and adds clearance. |
| cf93351c4 | keep | Cx latest pointer position accessor and event tracking. |
| 78a6d9761 | adapt | center_caption_label opt-out migrated to new Window implementation. |
| adbca8b4e | drop | Official Metal upload allocator already uses capacity, retirement safety, and pooled backings. |
| 3f624a81b | adapt | Public screenshot request API retained using official screen_capture backend. |
| 2178e6b6c | adapt | MAKEPAD_SCREENSHOT_PATH compatibility moved to shared screenshot response path. |
| 327582c54 | keep | Popup menus use begin_overlay_last. |
| 4fb3236dc | adapt | Application response polling uses one-shot screen_capture sink; retain official Metal/D3D11/GL/Vulkan readbacks. |
| ecb6d7077 | adapt | External document tokens/decorations retained with official DocumentLayout derives. |
| b141244f8 | adapt | Layered diagnostics retained alongside official diff decoration variants. |
| 70894417e | adapt | Window capture filters use official screen_capture window identity. Keep official Windows beat-credit pacing, which supersedes the old isolated wait relocation. |
| fc4523983 | drop / out of scope | Official live_rect_area and live_instance_data already guard stale slots. Legacy Windows headless JIT is outside widgets consumer migration; official replaced headless with gpusim. Original implementation remains in backup. |
| 2f206ada0 | adapt | Apply view shift to existing safe Area conversions; use those conversions in TextInput hit testing. |
| 52ff2d5f4 | keep | SVG authored viewport opt-in preserved. |
| 2dd011391 | adapt | Focus colors take precedence over hover in new buttons and text shader. |
| f91bbfdd8 | keep | Packaged resource resolver and Windows client-size synchronization retained. |
| 18e0bfa63 | adapt | Stable overlay slots and reorder tracking ported to new draw storage. Official already lazily formats append tracing. Original workspace manifest additions are obsolete. |

Regression coverage added for shifted/clipped input coordinates and alternating last overlays retaining bounded storage.

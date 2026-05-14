use std::path::{Path, PathBuf};

use gpui::*;

/// Raw captured frame data.
pub struct CapturedFrame {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// BGRA8 pixel data, 4 bytes per pixel, row-major.
    pub data: Vec<u8>,
}

impl CapturedFrame {
    /// Save the frame as a PNG file. Converts BGRA to RGBA internally.
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), crate::PreviewError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut rgba = self.data.clone();
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2); // B <-> R
        }
        image::save_buffer(
            path,
            &rgba,
            self.width,
            self.height,
            image::ColorType::Rgba8,
        )?;
        Ok(())
    }
}

/// A batch of component captures to process in a single application run.
pub struct CaptureSession {
    requests: Vec<CaptureRequest>,
}

struct CaptureRequest {
    make_element: Box<dyn Fn() -> AnyElement>,
    size: Size<Pixels>,
    path: PathBuf,
}

impl CaptureSession {
    /// Queue a component for capture as a PNG.
    pub fn png<C: RenderOnce + Clone + 'static>(
        &mut self,
        component: C,
        size: Size<Pixels>,
        path: impl Into<PathBuf>,
    ) -> &mut Self {
        self.requests.push(CaptureRequest {
            make_element: Box::new(move || {
                IntoElement::into_any_element(Component::new(component.clone()))
            }),
            size,
            path: path.into(),
        });
        self
    }
}

/// View wrapper that renders a type-erased element.
struct CaptureView {
    make_element: Box<dyn Fn() -> AnyElement>,
}

impl Render for CaptureView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        (self.make_element)()
    }
}

/// Run a capture session. Opens temporary windows, renders components,
/// captures frames to PNG, and exits. Blocks until all captures are complete.
///
/// ```ignore
/// gpui_preview::capture(gpui_component_assets::Assets, |s| {
///     s.png(Button::default(), size(px(200.), px(40.)), "button.png");
///     s.png(Card::default(), size(px(300.), px(200.)), "card.png");
/// });
/// ```
pub fn capture(assets: impl AssetSource + 'static, build: impl FnOnce(&mut CaptureSession)) {
    let mut session = CaptureSession {
        requests: Vec::new(),
    };
    build(&mut session);

    if session.requests.is_empty() {
        return;
    }

    let app = Application::new().with_assets(assets);
    app.run(move |cx| {
        gpui_component::init(cx);

        let requests = session.requests;
        cx.spawn(async move |cx| {
            for req in requests {
                let size = req.size;
                let bounds = Bounds::new(point(px(0.), px(0.)), size);

                let make_element = req.make_element;
                let options = WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    focus: false,
                    ..Default::default()
                };

                let handle = cx.open_window(options, |_window, cx| {
                    cx.new(|_cx| CaptureView {
                        make_element: Box::new(make_element),
                    })
                })?;

                // Wait for the GPU to render a frame
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(200))
                    .await;

                handle.update(cx, |_view, window, _cx| {
                    if let Some((w, h, data)) = window.capture_frame() {
                        let frame = CapturedFrame {
                            width: w,
                            height: h,
                            data,
                        };
                        match frame.save_png(&req.path) {
                            Ok(()) => println!("Saved {}", req.path.display()),
                            Err(e) => eprintln!("Failed to save {}: {e}", req.path.display()),
                        }
                    } else {
                        eprintln!("capture_frame returned None for {}", req.path.display());
                    }
                })?;
            }

            cx.update(|cx| cx.quit())?;
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

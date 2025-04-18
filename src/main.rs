// Prevent console window in addition to Slint window in Windows release builds
// when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;

slint::slint! {
    export component Ui inherits Window {
        in property <image> canvas_source <=> canvas.source;

        out property <length> canvas-width <=> canvas.width;
        out property <length> canvas-height <=> canvas.height;

        canvas := Image {
            width: 800px;
            height: 600px;
        }
    }
}

type Pixel = slint::Rgb8Pixel;

mod colors {
    use super::Pixel;
    pub(crate) const BLUE: Pixel = Pixel::new(0x60, 0x60, 0xff);
    pub(crate) const RED: Pixel = Pixel::new(0xff, 0x60, 0x60);
}

type CanvasBuffer = slint::SharedPixelBuffer<Pixel>;

trait UiExts {
    fn new_canvas_buffer(&self) -> CanvasBuffer;
    fn set_canvas_from_buffer(&self, buffer: CanvasBuffer);
    fn update_canvas(&self, f: impl FnOnce(&mut CanvasBuffer));
}

impl UiExts for Ui {
    fn new_canvas_buffer(&self) -> CanvasBuffer {
        let width = self.get_canvas_width() as u32;
        let height = self.get_canvas_height() as u32;
        CanvasBuffer::new(width, height)
    }

    fn set_canvas_from_buffer(&self, buffer: CanvasBuffer) {
        let source = slint::Image::from_rgb8(buffer);
        self.set_canvas_source(source);
    }

    fn update_canvas(&self, f: impl FnOnce(&mut CanvasBuffer)) {
        let mut buffer = self.new_canvas_buffer();
        f(&mut buffer);
        self.set_canvas_from_buffer(buffer);
    }
}

trait CanvasBufferExts {
    fn iter_pixels(
        &mut self,
        width: usize,
        f: impl FnMut(&mut Pixel, (usize, usize)),
    );
}

impl CanvasBufferExts for CanvasBuffer {
    fn iter_pixels(
        &mut self,
        width: usize,
        mut f: impl FnMut(&mut Pixel, (usize, usize)),
    ) {
        for (i, pixel) in self.make_mut_slice().iter_mut().enumerate() {
            f(pixel, (i % width, i / width));
        }
    }
}

struct SetTimeout(pub(crate) std::time::Duration);

impl SetTimeout {
    fn from_millis(millis: u64) -> Self {
        Self(std::time::Duration::from_millis(millis))
    }

    fn run(self, mut callback: impl FnMut() + 'static) {
        callback();
        let timer = Box::leak(Box::new(slint::Timer::default()));
        timer.start(slint::TimerMode::Repeated, self.0, callback);
    }
}

fn main() -> Result<()> {
    let ui = Ui::new()?;

    SetTimeout::from_millis(750).run({
        let ui = ui.as_weak();
        move || {
            ui.upgrade().unwrap().update_canvas(|buffer| {
                use std::sync::atomic::*;

                static X: AtomicBool = AtomicBool::new(true);

                let x = X.fetch_xor(true, Ordering::Relaxed);
                let (color_bg, color_dot) = if x {
                    (colors::BLUE, colors::RED)
                } else {
                    (colors::RED, colors::BLUE)
                };

                let width = buffer.width() as usize;
                let height = buffer.height() as usize;
                buffer.iter_pixels(width, move |pixel, (x, y)| {
                    let x = x as isize - width as isize / 2;
                    let y = y as isize - height as isize / 2;
                    let dist2 = x * x + y * y;
                    const R: isize = 64;
                    *pixel = if dist2 < R * R { color_dot } else { color_bg };
                });
            });
        }
    });

    ui.run()?;

    Ok(())
}

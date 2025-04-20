// Prevent console window in addition to Slint window in Windows release builds
// when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Mutex, atomic::*};

use anyhow::Result;
use tracing::level_filters;
use tracing_subscriber::{
    Layer, layer::SubscriberExt, util::SubscriberInitExt,
};

slint::slint! {
    export component Ui inherits Window {
        in property <image> canvas_source <=> canvas.source;

        out property <length> canvas-width <=> canvas.width;
        out property <length> canvas-height <=> canvas.height;
        out property <bool> canvas-has-hover <=> touch-area.has-hover;

        callback render(duration, length, length);
        public function tick() {
            render(animation-tick(), touch-area.mouse-x, touch-area.mouse-y);
        }

        touch-area := TouchArea {
            changed mouse-x => { root.tick(); }
            changed mouse-y => { root.tick(); }

            canvas := Image {
                width: 800px;
                height: 600px;
            }
        }
    }
}

type Pixel = slint::Rgb8Pixel;

mod colors {
    use super::Pixel;
    pub(crate) const BLACK: Pixel = Pixel::new(0x00, 0x00, 0x00);
    pub(crate) const BLUE: Pixel = Pixel::new(0x60, 0x60, 0xff);
    pub(crate) const RED: Pixel = Pixel::new(0xff, 0x60, 0x60);
}

type CanvasBuffer = slint::SharedPixelBuffer<Pixel>;

static BUFFER: AtomicPtr<CanvasBuffer> = AtomicPtr::new(std::ptr::null_mut());
// static BUFFER2: AtomicPtr<CanvasBuffer> = AtomicPtr::new(std::ptr::null_mut());

fn set_buffers(buffer: CanvasBuffer, buffer2: CanvasBuffer) {
    let buffer = Box::leak(Box::new(buffer));
    // let buffer2 = Box::leak(Box::new(buffer2));
    BUFFER.store(buffer, Ordering::Relaxed);
    // BUFFER2.store(buffer2, Ordering::Relaxed);
}

#[tracing::instrument]
fn get_and_swap_buffers_mut() -> &'static mut CanvasBuffer {
    // let buffer2 = BUFFER2.load(Ordering::Relaxed);
    // unsafe { &mut *BUFFER.swap(buffer2, Ordering::Relaxed) }
    unsafe { &mut *BUFFER.load(Ordering::Relaxed) }
}

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

    #[tracing::instrument(skip_all)]
    fn update_canvas(&self, f: impl FnOnce(&mut CanvasBuffer)) {
        // let buffer = get_and_swap_buffers_mut();
        static BUF: Mutex<Option<CanvasBuffer>> = Mutex::new(None);
        let mut buffer = BUF.lock().unwrap();
        if buffer.is_none() {
            *buffer = Some(self.new_canvas_buffer());
        }
        let buffer = buffer.as_mut().unwrap();
        f(buffer);
        self.set_canvas_from_buffer(buffer.clone());
    }
}

trait CanvasBufferExts {
    fn coords(&self, x: isize, y: isize) -> (isize, isize);
    fn iter_pixels(
        &mut self,
        width: isize,
        f: impl FnMut(&mut Pixel, (isize, isize)),
    );
}

impl CanvasBufferExts for CanvasBuffer {
    fn coords(&self, x: isize, y: isize) -> (isize, isize) {
        let x = x - self.width() as isize / 2;
        let y = self.height() as isize / 2 - y;
        (x, y)
    }

    fn iter_pixels(
        &mut self,
        width: isize,
        mut f: impl FnMut(&mut Pixel, (isize, isize)),
    ) {
        let half_width = self.width() as isize / 2;
        let half_height = self.height() as isize / 2;
        for (i, pixel) in self.make_mut_slice().iter_mut().enumerate() {
            let x = (i as isize % width) - half_width;
            let y = half_height - (i as isize / width);
            f(pixel, (x, y));
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
    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(level_filters::LevelFilter::INFO.into())
        .from_env_lossy();

    let tracy_layer = tracing_tracy::TracyLayer::default();

    tracing_subscriber::registry()
        .with(stdout_log.with_filter(env_filter))
        .with(tracy_layer)
        .init();

    let ui = Ui::new()?;

    set_buffers(ui.new_canvas_buffer(), ui.new_canvas_buffer());
    {
        let buffer = unsafe { &*BUFFER.load(Ordering::Relaxed) };
        println!("BUFFER: {:p}", buffer);
    }

    static DOT_COLOR: AtomicBool = AtomicBool::new(true);

    SetTimeout::from_millis(750).run({
        let ui = ui.as_weak();
        move || {
            let _frame = tracy_client::non_continuous_frame!("change color");
            DOT_COLOR.fetch_xor(true, Ordering::Relaxed);
            let ui = ui.upgrade().unwrap();
            ui.invoke_tick();
        }
    });

    ui.on_render({
        let ui = ui.as_weak();
        move |t, mouse_x, mouse_y| {
            let _span = tracy_client::span!("render");

            static LAST_T: AtomicI64 = AtomicI64::new(0);
            let last_t = LAST_T.load(Ordering::Relaxed);
            if t <= last_t || t - last_t < 24 {
                return;
            }
            LAST_T.store(t, Ordering::Relaxed);

            let ui = ui.upgrade().unwrap();

            let hover = ui.get_canvas_has_hover();

            ui.update_canvas(|buffer| {
                let _span = tracy_client::span!("update");

                let (mouse_x, mouse_y) =
                    buffer.coords(mouse_x as isize, mouse_y as isize);

                let color_dot = if DOT_COLOR.load(Ordering::Relaxed) {
                    colors::BLUE
                } else {
                    colors::RED
                };

                println!("{:p}", buffer);

                let width = buffer.width() as isize;
                buffer.iter_pixels(width, |pixel, (x, y)| {
                    let (x, y) = (x - mouse_x, y - mouse_y);
                    const R: isize = 32;
                    *pixel = if hover && x * x + y * y < R * R {
                        color_dot
                    } else {
                        colors::BLACK
                    };
                });
            });
        }
    });

    tracing::info!("Running application");
    ui.run()?;

    Ok(())
}

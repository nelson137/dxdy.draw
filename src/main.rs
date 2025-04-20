use std::{
    f64::consts::TAU,
    sync::{
        RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::Result;
use gtk::{cairo, gdk, glib, prelude::*};
use tracing::level_filters;
use tracing_subscriber::{
    Layer, layer::SubscriberExt, util::SubscriberInitExt,
};

const APP_ID: &str = "com.nelsonearle.dxdy.draw";

#[derive(Clone, Copy)]
struct Pos {
    x: f64,
    y: f64,
}

impl Pos {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

static CURSOR_POSITION: RwLock<Option<Pos>> = RwLock::new(None);

static CURSOR_COLOR: AtomicBool = AtomicBool::new(true);

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

    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_activate(cb_activate);

    let exit_code = app.run_with_args(&[] as &[&str]);
    if exit_code != glib::ExitCode::SUCCESS {
        eprintln!("{exit_code:?}");
    }

    Ok(())
}

fn eat_err(r: Result<()>) {
    if let Err(err) = r {
        glib::g_error!("dxdy.draw", "{err}");
    }
}

fn cb_activate(app: &gtk::Application) {
    // Drawing Area

    let drawing_area = gtk::DrawingArea::builder()
        .content_width(800)
        .content_height(600)
        .build();

    // Window

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("DxDy Draw")
        .default_width(800)
        .default_height(600)
        .resizable(false)
        .child(&drawing_area)
        .build();

    // Draw

    drawing_area.set_draw_func(glib::clone!(
        move |widget, ctx, w, h| eat_err(draw(widget, ctx, w, h))
    ));

    // Key Press

    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed(glib::clone!(
        #[weak]
        app,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |controller, keyval, keycode, modifier| {
            cb_key_pressed(app, controller, keyval, keycode, modifier)
        }
    ));
    window.add_controller(key_controller);

    // Cursor Position

    fn get_pointer_position(
        window: gtk::ApplicationWindow,
    ) -> Option<(Pos, gdk::ModifierType)> {
        let display = gdk::Display::default().unwrap();
        let pointer = display.default_seat().unwrap().pointer().unwrap();
        let surface = window.root().unwrap().surface().unwrap();
        surface
            .device_position(&pointer)
            .map(|(x, y, modt)| (Pos::new(x, y), modt))
    }

    glib::timeout_add_local(
        std::time::Duration::from_millis(20),
        glib::clone!(
            #[weak]
            window,
            #[weak]
            drawing_area,
            #[upgrade_or]
            glib::ControlFlow::Continue,
            move || {
                match get_pointer_position(window) {
                    Some((pos, _)) => {
                        *CURSOR_POSITION.write().unwrap() = Some(pos);
                    }
                    None => {
                        *CURSOR_POSITION.write().unwrap() = None;
                    }
                }
                drawing_area.queue_draw();
                glib::ControlFlow::Continue
            }
        ),
    );

    // Cursor Color

    glib::timeout_add_local(
        std::time::Duration::from_millis(750),
        glib::clone!(move || {
            CURSOR_COLOR.fetch_xor(true, Ordering::Relaxed);
            glib::ControlFlow::Continue
        }),
    );

    // Present

    window.present();
}

fn cb_key_pressed(
    app: gtk::Application,
    _controller: &gtk::EventControllerKey,
    keyval: gdk::Key,
    _keycode: u32,
    modifier: gdk::ModifierType,
) -> glib::Propagation {
    if modifier == gdk::ModifierType::META_MASK && keyval == gdk::Key::q {
        app.quit();
    }

    glib::Propagation::Proceed
}

mod colors {
    use gtk::gdk::RGBA;

    const fn f(b: u8) -> f32 {
        b as f32 / u8::MAX as f32
    }

    pub(crate) static BLUE: RGBA = RGBA::new(f(0x60), f(0x60), f(0xff), 1.);
    pub(crate) static RED: RGBA = RGBA::new(f(0xff), f(0x60), f(0x60), 1.);

    pub(crate) static BG: RGBA = RGBA::new(0.2, 0.2, 0.2, 1.);
    pub(crate) static CURSOR1: RGBA = BLUE;
    pub(crate) static CURSOR2: RGBA = RED;
}

mod sizes {
    pub(crate) static CURSOR_RADIUS: f64 = 32.;
}

fn draw(
    _widget: &gtk::DrawingArea,
    ctx: &cairo::Context,
    width: i32,
    height: i32,
) -> Result<()> {
    ctx.set_source_color(&colors::BG);
    ctx.rectangle(0.0, 0.0, width as f64, height as f64);
    ctx.fill()?;

    if let Some(pos) = *CURSOR_POSITION.read().unwrap() {
        ctx.arc(pos.x, pos.y, sizes::CURSOR_RADIUS, 0., TAU);
        ctx.set_source_color(if CURSOR_COLOR.load(Ordering::Relaxed) {
            &colors::CURSOR1
        } else {
            &colors::CURSOR2
        });
        ctx.fill()?;
    }

    Ok(())
}

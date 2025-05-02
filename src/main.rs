use std::{
    f64::consts::TAU,
    sync::{
        RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use anyhow::Result;
use gtk::{cairo, gdk, glib, prelude::*};
use tracing::level_filters;
use tracing_subscriber::{
    Layer, layer::SubscriberExt, util::SubscriberInitExt,
};

mod algorithm;
mod pos;
mod shape;

use pos::*;
use shape::*;

const APP_ID: &str = "com.nelsonearle.dxdy.draw";

static CURSOR_POSITION: RwLock<Option<Pos>> = RwLock::new(None);

static CURSOR_COLOR: AtomicBool = AtomicBool::new(true);

static CURRENT_SHAPE: RwLock<Shape> = RwLock::new(Shape::new());

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

    // Drag Gesture

    let gesture_drag = gtk::GestureDrag::new();
    gesture_drag.set_button(gdk::BUTTON_PRIMARY);

    gesture_drag.connect_drag_begin(|gesture, x, y| {
        gesture.set_state(gtk::EventSequenceState::Claimed);
        *CURRENT_SHAPE.write().unwrap() = Shape::from_pos(x, y);
    });

    static DRAG_APP_START: std::sync::LazyLock<std::time::Instant> =
        std::sync::LazyLock::new(std::time::Instant::now);
    _ = *DRAG_APP_START;

    static DRAG_LAST_UPDATE: AtomicU64 = AtomicU64::new(0);

    gesture_drag.connect_drag_update(|gesture, _dx, _dy| {
        gesture.set_state(gtk::EventSequenceState::Claimed);

        let t = DRAG_APP_START.elapsed().as_millis() as u64;
        if t - DRAG_LAST_UPDATE.load(Ordering::Relaxed) < 50 {
            return;
        }
        DRAG_LAST_UPDATE.store(t, Ordering::Relaxed);

        if let Some((dx, dy)) = gesture.offset() {
            let offset = PosOffset::new(dx, dy);
            let mut current_shape = CURRENT_SHAPE.write().unwrap();

            let last_offset = current_shape.last_offset();
            let dist_to_last = (offset - last_offset).dist2();
            if dist_to_last < 400. {
                return;
            }

            current_shape.next_vertex_at(offset);
        }
    });

    gesture_drag.connect_drag_end(|gesture, _dx, _dy| {
        gesture.set_state(gtk::EventSequenceState::Claimed);
        if let Some((dx, dy)) = gesture.offset() {
            let mut current_shape = CURRENT_SHAPE.write().unwrap();
            current_shape.next_vertex(dx, dy);
            ALL_SHAPES.write().unwrap().push(current_shape.clone());
        }
    });

    window.add_controller(gesture_drag);

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
    } else if keyval == gdk::Key::BackSpace {
        ALL_SHAPES.write().unwrap().clear();
        *CURRENT_SHAPE.write().unwrap() = Shape::new();
    }

    glib::Propagation::Proceed
}

mod colors {
    use gtk::gdk::RGBA;

    const fn f(b: u8) -> f32 {
        b as f32 / u8::MAX as f32
    }

    pub(crate) static WHITE: RGBA = RGBA::new(f(0xff), f(0xff), f(0xff), 1.);
    pub(crate) static BLUE: RGBA = RGBA::new(f(0x60), f(0x60), f(0xff), 1.);
    pub(crate) static RED: RGBA = RGBA::new(f(0xff), f(0x60), f(0x60), 1.);

    pub(crate) static BG: RGBA = RGBA::new(0.2, 0.2, 0.2, 1.);
    pub(crate) static CURSOR1: RGBA = BLUE;
    pub(crate) static CURSOR2: RGBA = RED;
}

mod sizes {
    pub(crate) static CURSOR_RADIUS: f64 = 4.;
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

    let (color, color_opposite) = if CURSOR_COLOR.load(Ordering::Relaxed) {
        (&colors::CURSOR1, &colors::CURSOR2)
    } else {
        (&colors::CURSOR2, &colors::CURSOR1)
    };

    ctx.set_source_color(color);

    if let Some(pos) = *CURSOR_POSITION.read().unwrap() {
        ctx.arc(pos.x, pos.y, sizes::CURSOR_RADIUS, 0., TAU);
        ctx.fill()?;
    }

    {
        let shape = CURRENT_SHAPE.read().unwrap();
        let start = shape.start();
        ctx.new_path();
        ctx.move_to(start.x, start.y);
        for offset in shape.verticies() {
            let x = start.x + offset.dx;
            let y = start.y + offset.dy;
            ctx.line_to(x, y);
        }
        ctx.stroke()?;
    }

    for shape in ALL_SHAPES.read().unwrap().iter() {
        let start = shape.start();

        ctx.set_source_color(color_opposite);
        ctx.set_line_width(4.);
        ctx.new_path();
        for offset in shape.verticies() {
            let x = start.x + offset.dx;
            let y = start.y + offset.dy;
            ctx.line_to(x, y);
        }
        ctx.close_path();
        ctx.stroke()?;

        ctx.set_source_color(&colors::WHITE);
        ctx.set_line_width(1.);
        for offset in shape.verticies() {
            let x = start.x + offset.dx;
            let y = start.y + offset.dy;
            ctx.arc(x, y, 1.5, 0., TAU);
            ctx.stroke()?;
        }
    }

    Ok(())
}

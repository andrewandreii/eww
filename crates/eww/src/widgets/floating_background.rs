use anyhow::{anyhow, Result};
use gtk::glib::{self, object_subclass, prelude::*, wrapper, Properties};
use gtk::{cairo, gdk, prelude::*, subclass::prelude::*};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::error_handling_ctx;

wrapper! {
    pub struct FloatingBackground(ObjectSubclass<FloatingBackgroundPriv>)
    @extends gtk::Bin, gtk::Container, gtk::Widget;
}

struct FloatingBackgroundState {
    margin: f64,
    radius: f64,
    color: gdk::RGBA,
}

#[derive(Properties)]
#[properties(wrapper_type = FloatingBackground)]
pub struct FloatingBackgroundPriv {
    #[property(get, set, nick = "Floating", blurb = "The floating state", default = true)]
    floating: RefCell<bool>,

    #[property(get, set, nick = "Max margin", blurb = "The maximum margin", minimum = 0f64, maximum = 100f64, default = 7f64)]
    max_margin: RefCell<f64>,

    #[property(get, set, nick = "Max radius", blurb = "The maximum radius", minimum = 0f64, maximum = 360f64, default = 5f64)]
    max_radius: RefCell<f64>,

    #[property(
        get,
        set,
        nick = "Floating opacity",
        blurb = "The opacity when floating",
        minimum = 0f64,
        maximum = 1f64,
        default = 0.8f64
    )]
    floating_opacity: RefCell<f64>,

    state: Rc<RefCell<FloatingBackgroundState>>,

    content: RefCell<Option<gtk::Widget>>,
}

impl Default for FloatingBackgroundPriv {
    fn default() -> Self {
        FloatingBackgroundPriv {
            floating: RefCell::new(false),
            max_margin: RefCell::new(7f64),
            max_radius: RefCell::new(5f64),
            floating_opacity: RefCell::new(0.8f64),
            state: Rc::new(RefCell::new(FloatingBackgroundState { margin: 0f64, radius: 0f64, color: gdk::RGBA::WHITE })),
            content: RefCell::new(None),
        }
    }
}

impl ObjectImpl for FloatingBackgroundPriv {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "floating" => {
                self.transition(value.get().unwrap());
            }
            "max-margin" => {
                self.max_margin.replace(value.get().unwrap());
            }
            "max-radius" => {
                self.max_radius.replace(value.get().unwrap());
            }
            "floating-opacity" => {
                self.floating_opacity.replace(value.get().unwrap());
            }
            x => panic!("Tried to set inexistant property of AnimatedBackground: {}", x,),
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        self.derived_property(id, pspec)
    }
}

impl FloatingBackgroundPriv {
    pub fn transition(&self, value: bool) {
        if *self.floating.borrow() == value {
            return;
        }
        self.floating.replace(value);

        let easing = |progress: f64, min: f64, max: f64| {
            return progress * progress * (max - min) + min;
        };

        let styles = self.obj().style_context();
        let bg_color: gdk::RGBA =
            styles.style_property_for_state("background-color", gtk::StateFlags::NORMAL).get().unwrap_or(gdk::RGBA::WHITE);

        let widget = self.obj().clone();
        let state = self.state.clone();
        RefCell::borrow_mut(&state).color = bg_color;
        let max_margin = *RefCell::borrow(&self.max_margin);
        let max_radius = *RefCell::borrow(&self.max_radius);
        let floating_opacity = *RefCell::borrow(&self.floating_opacity);

        let mut progress = 0f64;
        glib::timeout_add_local(Duration::from_millis(10), move || {
            let mut state = RefCell::borrow_mut(&state);
            progress += 0.1;

            let prog = if !value { 1f64 - progress } else { progress };

            state.margin = easing(prog, 0f64, max_margin);
            state.radius = easing(prog, 0f64, max_radius);
            state.color.set_alpha(easing(prog, bg_color.alpha(), floating_opacity));

            widget.queue_draw();

            if progress >= 1f64 {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
}

#[object_subclass]
impl ObjectSubclass for FloatingBackgroundPriv {
    type ParentType = gtk::Bin;
    type Type = FloatingBackground;

    const NAME: &'static str = "FloatingBackground";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("floating-background");
    }
}

impl Default for FloatingBackground {
    fn default() -> Self {
        Self::new()
    }
}

impl FloatingBackground {
    pub fn new() -> Self {
        glib::Object::new::<Self>()
    }
}

impl ContainerImpl for FloatingBackgroundPriv {
    fn add(&self, widget: &gtk::Widget) {
        if let Some(content) = &*self.content.borrow() {
            // TODO: Handle this error when populating children widgets instead
            error_handling_ctx::print_error(anyhow!("Error, trying to add multiple children to a floating-background widget"));
            self.parent_remove(content);
        }
        self.parent_add(widget);
        self.content.replace(Some(widget.clone()));
    }
}

impl BinImpl for FloatingBackgroundPriv {}

impl WidgetImpl for FloatingBackgroundPriv {
    fn draw(&self, cr: &cairo::Context) -> glib::Propagation {
        let res: Result<()> = (|| {
            let FloatingBackgroundState { margin, radius, color } = *RefCell::borrow(&self.state);

            let styles = self.obj().style_context();
            let padding = styles.padding(gtk::StateFlags::NORMAL);

            let win = self.obj().window().unwrap();
            let height = win.height() as f64;
            let width = win.width() as f64;

            cr.save()?;

            cr.set_source_rgba(color.red(), color.green(), color.blue(), color.alpha());
            cr.new_sub_path();
            cr.arc(margin + radius, margin + radius, radius, 180f64.to_radians(), 270f64.to_radians());
            cr.arc(width - radius - margin, margin + radius, radius, 270f64.to_radians(), 0f64.to_radians());
            cr.arc(width - radius - margin, height - radius, radius, 0f64.to_radians(), 90f64.to_radians());
            cr.arc(margin + radius, height - radius, radius, 90f64.to_radians(), 180f64.to_radians());
            cr.close_path();

            cr.fill()?;

            cr.restore()?;

            if let Some(child) = &*self.content.borrow() {
                cr.save()?;

                let padding_start = margin as i32 + padding.left as i32;
                let padding_top = margin as i32 + padding.top as i32;

                child.set_margin_top(padding_top);
                child.set_margin_start(padding_start);
                child.set_margin_end(padding_start);

                // Child widget
                self.obj().propagate_draw(child, cr);

                cr.reset_clip();
                cr.restore()?;
            }
            Ok(())
        })();

        if let Err(error) = res {
            error_handling_ctx::print_error(error)
        };

        glib::Propagation::Proceed
    }
}

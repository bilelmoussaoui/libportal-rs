use std::sync::Arc;

use ashpd::desktop::screenshot::{
    AsyncScreenshotProxy, Color, PickColorOptions, Screenshot, ScreenshotOptions,
};
use ashpd::zbus;
use ashpd::{Response, WindowIdentifier};
use futures::lock::Mutex;
use futures::FutureExt;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::widgets::ColorWidget;

mod imp {
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;

    use super::*;

    #[derive(Debug, CompositeTemplate, Default)]
    #[template(resource = "/com/belmoussaoui/ashpd/demo/screenshot.ui")]
    pub struct ScreenshotPage {
        #[template_child]
        pub interactive_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub modal_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub color_widget: TemplateChild<ColorWidget>,
        #[template_child]
        pub screenshot_photo: TemplateChild<gtk::Picture>,
        #[template_child]
        pub response_group: TemplateChild<adw::PreferencesGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ScreenshotPage {
        const NAME: &'static str = "ScreenshotPage";
        type Type = super::ScreenshotPage;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action(
                "screenshot.pick-color",
                None,
                move |page, _action, _target| {
                    page.pick_color();
                },
            );
            klass.install_action(
                "screenshot.screenshot",
                None,
                move |page, _action, _target| {
                    page.screenshot();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for ScreenshotPage {
        fn constructed(&self, obj: &Self::Type) {
            self.screenshot_photo.set_overflow(gtk::Overflow::Hidden);
        }
    }
    impl WidgetImpl for ScreenshotPage {}
    impl BinImpl for ScreenshotPage {}
}

glib::wrapper! {
    pub struct ScreenshotPage(ObjectSubclass<imp::ScreenshotPage>) @extends gtk::Widget, adw::Bin;
}

impl ScreenshotPage {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Failed to create a ScreenshotPage")
    }

    pub fn pick_color(&self) {
        let self_ = imp::ScreenshotPage::from_instance(self);

        let ctx = glib::MainContext::default();
        let color_widget = self_.color_widget.get();
        ctx.spawn_local(async move {
            if let Ok(Response::Ok(color)) = pick_color(WindowIdentifier::default()).await {
                color_widget.set_rgba(color.into());
            }
        });
    }

    pub fn screenshot(&self) {
        let self_ = imp::ScreenshotPage::from_instance(self);

        let interactive = self_.interactive_switch.get_active();
        let modal = self_.modal_switch.get_active();
        let screenshot_photo = self_.screenshot_photo.get();
        let response_group = self_.response_group.get();

        let ctx = glib::MainContext::default();
        ctx.spawn_local(async move {
            if let Ok(Response::Ok(screenshot)) =
                screenshot(WindowIdentifier::default(), interactive, modal).await
            {
                let file = gio::File::new_for_uri(&screenshot.uri);
                screenshot_photo.set_file(Some(&file));
                response_group.show();
            }
        });
    }
}

pub async fn pick_color(window_identifier: WindowIdentifier) -> zbus::Result<Response<Color>> {
    let connection = zbus::azync::Connection::new_session().await?;
    let proxy = AsyncScreenshotProxy::new(&connection)?;
    let request = proxy
        .pick_color(window_identifier, PickColorOptions::default())
        .await?;

    let (sender, receiver) = futures::channel::oneshot::channel();

    let sender = Arc::new(Mutex::new(Some(sender)));
    let signal_id = request
        .connect_response(move |response: Response<Color>| {
            let s = sender.clone();
            async move {
                if let Some(m) = s.lock().await.take() {
                    let _ = m.send(response);
                }
                Ok(())
            }
            .boxed()
        })
        .await?;

    while request.next_signal().await?.is_some() {}
    request.disconnect_signal(signal_id).await?;

    let color = receiver.await.unwrap();
    Ok(color)
}

pub async fn screenshot(
    window_identifier: WindowIdentifier,
    interactive: bool,
    modal: bool,
) -> zbus::Result<Response<Screenshot>> {
    let connection = zbus::azync::Connection::new_session().await?;
    let proxy = AsyncScreenshotProxy::new(&connection)?;
    let request = proxy
        .screenshot(
            window_identifier,
            ScreenshotOptions::default()
                .interactive(interactive)
                .modal(modal),
        )
        .await?;

    let (sender, receiver) = futures::channel::oneshot::channel();

    let sender = Arc::new(Mutex::new(Some(sender)));
    let signal_id = request
        .connect_response(move |response: Response<Screenshot>| {
            let s = sender.clone();
            async move {
                if let Some(m) = s.lock().await.take() {
                    let _ = m.send(response);
                }
                Ok(())
            }
            .boxed()
        })
        .await?;

    while request.next_signal().await?.is_some() {}
    request.disconnect_signal(signal_id).await?;

    let screenshot = receiver.await.unwrap();
    Ok(screenshot)
}

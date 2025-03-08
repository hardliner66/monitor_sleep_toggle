use tao::{
    event::Event,
    event_loop::{ControlFlow, EventLoopBuilder},
};
use tray_icon::{
    MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem},
};

#[derive(Debug)]
enum UserEvent {
    TrayIconEvent(tray_icon::TrayIconEvent),
    MenuEvent(tray_icon::menu::MenuEvent),
}

const HERE_ICON: &[u8] = include_bytes!("../assets/here.png");
const AWAY_ICON: &[u8] = include_bytes!("../assets/away.png");

fn set_monitor_timeout(timeout: u64) {
    let output = std::process::Command::new("powercfg.exe")
        .args([
            "/Change".to_string(),
            "monitor-timeout-ac".to_string(),
            timeout.to_string(),
        ])
        .output()
        .expect("Failed to execute command");
    if !output.status.success() {
        eprintln!("Failed to set monitor timeout");
    }
}

#[derive(Debug, PartialEq)]
enum State {
    Here,
    Away,
}

fn main() -> anyhow::Result<()> {
    let here_icon = load_icon(HERE_ICON);
    let away_icon = load_icon(AWAY_ICON);

    set_monitor_timeout(0);

    let mut state = State::Here;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // set a tray event handler that forwards the event and wakes up the event loop
    let proxy = event_loop.create_proxy();
    TrayIconEvent::set_event_handler(Some(move |event| {
        proxy
            .send_event(UserEvent::TrayIconEvent(event))
            .expect("Couldn't send event");
    }));

    // set a menu event handler that forwards the event and wakes up the event loop
    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event| {
        proxy
            .send_event(UserEvent::MenuEvent(event))
            .expect("Couldn't send event");
    }));

    let tray_menu = Menu::new();

    let quit_i = MenuItem::new("Quit", true, None);
    tray_menu.append_items(&[&quit_i])?;

    let mut tray_icon = None;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
                // We create the icon once the event loop is actually running
                // to prevent issues like https://github.com/tauri-apps/tray-icon/issues/90
                tray_icon = Some(
                    TrayIconBuilder::new()
                        .with_menu_on_left_click(false)
                        .with_menu(Box::new(tray_menu.clone()))
                        .with_tooltip("Monitor Control")
                        .with_icon(here_icon.clone())
                        .build()
                        .unwrap(),
                );
            }

            Event::UserEvent(UserEvent::TrayIconEvent(TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Down,
                ..
            })) => match state {
                State::Here => {
                    tray_icon
                        .as_mut()
                        .unwrap()
                        .set_icon(Some(away_icon.clone()))
                        .unwrap();
                    set_monitor_timeout(1);
                    state = State::Away;
                }
                State::Away => {
                    tray_icon
                        .as_mut()
                        .unwrap()
                        .set_icon(Some(here_icon.clone()))
                        .unwrap();
                    set_monitor_timeout(0);
                    state = State::Here;
                }
            },

            Event::UserEvent(UserEvent::MenuEvent(event)) => {
                if event.id == quit_i.id() {
                    set_monitor_timeout(0);
                    tray_icon.take();
                    *control_flow = ControlFlow::Exit;
                }
            }

            _ => {}
        }
    })
}

fn load_icon(data: &[u8]) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(data)
            .expect("Failed to load image")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

use nwg::NativeUi;
use std::sync::mpsc;

use crate::app::Cmd;

const ICON_DATA: &[u8] = std::include_bytes!("./../resources/icon.png");

// pub fn begin(tx: std::sync::mpsc::Sender<Cmd>, rx: std::sync::mpsc::Receiver<Cmd>) {
pub fn begin(tx: mpsc::Sender<Cmd>, rx: mpsc::Receiver<Cmd>) {
    nwg::init().expect("Failed to init Native Windows GUI");
    let _ui = Main::build_ui(Main::new(tx, rx)).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}

pub struct Main {
    window: nwg::Window,
    menu_ui: MenuUi,
    layout: nwg::FlexboxLayout,
    icon: nwg::Icon,
    hosts_file_dialog: nwg::FileDialog,
    options_frame: nwg::Frame,
    options: Options,
    tray: Systray,
    preview_ui: PreviewUi,
    actions_ui: ActionsUi,
    actions_frame: nwg::Frame,
    status: nwg::StatusBar,
    tx: mpsc::Sender<Cmd>,
    rx: mpsc::Receiver<Cmd>,
}

#[derive(Default)]
pub struct ActionsUi {
    layout: nwg::FlexboxLayout,
    preview_button: nwg::Button,
    write_button: nwg::Button,
}

#[derive(Default)]
pub struct Options {
    layout: nwg::FlexboxLayout,
    hosts_path_file_button: nwg::Button,
    hosts_path_row_frame: nwg::Frame,
    hosts_path_row: nwg::FlexboxLayout,
    hosts_path_input: nwg::TextInput,
    hosts_path_label: nwg::Label,

    view_hosts_button: nwg::Button,

    names_ui_frame: nwg::Frame,
    names_ui: NamesUi,
}

#[derive(Default)]
pub struct PreviewUi {
    window: nwg::Window,
    layout: nwg::FlexboxLayout,
    preview: nwg::TextBox,
}

#[derive(Default)]
pub struct MenuUi {
    main: nwg::Menu,
    save: nwg::MenuItem,
    sep: nwg::MenuSeparator,
    quit: nwg::MenuItem,
}

#[derive(Default)]
pub struct HostsUi {}

#[derive(Default)]
pub struct NamesUi {
    layout: nwg::FlexboxLayout,
    input_frame: nwg::Frame,
    input_row: nwg::FlexboxLayout,
    list_frame: nwg::Frame,
    list_row: nwg::FlexboxLayout,
    names_list: nwg::ListBox<String>,
    names_input: nwg::TextInput,
    names_add: nwg::Button,
    names_remove: nwg::Button,
}

#[derive(Default)]
pub struct Systray {
    icon: nwg::Icon,
    tray: nwg::TrayNotification,
    tray_menu: nwg::Menu,
    tray_run: nwg::MenuItem,
    tray_open: nwg::MenuItem,
    tray_sep: nwg::MenuSeparator,
    tray_exit: nwg::MenuItem,
}

pub mod ui {
    use super::*;
    use main as lib;
    use nwg::{
        self,
        stretch::geometry::{Rect, Size},
        stretch::style::{self, Dimension},
        Event, NativeUi, PartialUi,
    };
    use std::{cell::RefCell, rc::Rc};

    pub struct MainUi {
        inner: Rc<Main>,
        default_handler: RefCell<Vec<nwg::EventHandler>>,
    }

    impl Main {
        pub fn new(tx: mpsc::Sender<Cmd>, rx: mpsc::Receiver<Cmd>) -> Self {
            Main {
                window: nwg::Window::default(),
                menu_ui: MenuUi::default(),
                layout: nwg::FlexboxLayout::default(),
                hosts_file_dialog: nwg::FileDialog::default(),
                icon: nwg::Icon::default(),
                tray: Systray::default(),
                options_frame: nwg::Frame::default(),
                options: Options::default(),
                preview_ui: PreviewUi::default(),
                actions_frame: nwg::Frame::default(),
                actions_ui: ActionsUi::default(),
                status: nwg::StatusBar::default(),
                tx: tx,
                rx: rx,
            }
        }

        fn update_buttons(&self, access: lib::Access) {
            self.actions_ui.write_button.set_enabled(access.write);
            self.actions_ui.preview_button.set_enabled(access.read);
            self.options.view_hosts_button.set_enabled(access.read);
        }
        fn on_init(&self) {
            self.tx.send(Cmd::OnInit).unwrap();
            match self.rx.recv() {
                Ok(Cmd::State(c)) => {
                    self.options.hosts_path_input.set_text(&c.hosts_path);
                    self.options
                        .names_ui
                        .names_list
                        .set_collection(c.names.to_owned());

                    let access = c.check_hosts_path();
                    if false == access.write {
                        self.status
                            .set_text(0, "Insufficient access to write to hosts file.");
                    }
                    self.update_buttons(access);
                }
                Ok(Cmd::Content(s)) => {
                    self.status.set_text(0, &s);
                }
                _ => (),
            }
        }

        fn open(&self) {
            self.window.set_visible(true);
        }

        fn add_name(&self) {
            let name = self.options.names_ui.names_input.text().to_owned();
            self.tx.send(Cmd::AddName(name)).unwrap();
            if let Ok(Cmd::State(c)) = self.rx.recv() {
                self.options
                    .names_ui
                    .names_list
                    .set_collection(c.names.to_owned());
            }
        }

        fn remove_name(&self) {
            let idx = match self.options.names_ui.names_list.selection() {
                Some(i) => i,
                None => return,
            };

            let name = self.options.names_ui.names_list.remove(idx);

            self.tx.send(Cmd::RemoveName(name)).unwrap();
            if let Ok(Cmd::State(c)) = self.rx.recv() {
                self.options
                    .names_ui
                    .names_list
                    .set_collection(c.names.to_owned());
            }
        }

        fn select_file(&self) {
            if true == self.hosts_file_dialog.run(Some(self.window.handle)) {
                match self.hosts_file_dialog.get_selected_item() {
                    Ok(s) => {
                        self.tx
                            .send(Cmd::SetHostsFile(s.to_str().unwrap().to_owned()))
                            .unwrap();

                        if let Ok(Cmd::State(c)) = self.rx.recv() {
                            self.options.hosts_path_input.set_text(&c.hosts_path);
                            self.status.set_text(0, "Updated hosts file path.");
                            self.update_buttons(c.check_hosts_path());
                        }
                    }
                    _ => (),
                }
            }
        }

        fn show_hosts_file(&self) {
            self.tx.send(Cmd::ReadFile).unwrap();
            if let Ok(Cmd::Content(s)) = self.rx.recv() {
                self.preview_ui.preview.set_text(&s);
                self.preview_ui.window.set_visible(true);
            }
        }

        fn show_preview(&self) {
            self.tx.send(Cmd::Preview).unwrap();
            if let Ok(Cmd::Content(s)) = self.rx.recv() {
                self.preview_ui.preview.set_text(&s);
                self.preview_ui.window.set_visible(true);
            }
        }

        fn save_config(&self) {
            self.tx.send(Cmd::SaveConfig).unwrap();
            if let Ok(Cmd::Content(s)) = self.rx.recv() {
                self.status.set_text(0, &s);
            }
        }

        fn write(&self) {
            self.tx.send(Cmd::Write).unwrap();
            if let Ok(Cmd::Content(s)) = self.rx.recv() {
                self.status.set_text(0, &s);
            }
        }

        fn show_menu(&self) {
            let (x, y) = nwg::GlobalCursor::position();
            self.tray.tray_menu.popup(x, y);
        }

        fn on_exit(&self) {
            self.tx.send(Cmd::Quit).unwrap();
            nwg::stop_thread_dispatch();
        }
    }

    impl PartialUi for MenuUi {
        fn build_partial<W: Into<nwg::ControlHandle>>(
            data: &mut MenuUi,
            parent: Option<W>,
        ) -> Result<(), nwg::NwgError> {
            let parent = parent.unwrap().into();

            nwg::Menu::builder()
                .text("Menu")
                .parent(&parent)
                .build(&mut data.main)?;

            nwg::MenuItem::builder()
                .text("Save Config")
                .parent(&data.main)
                .build(&mut data.save)?;

            nwg::MenuSeparator::builder()
                .parent(&data.main)
                .build(&mut data.sep)?;

            nwg::MenuItem::builder()
                .text("Quit")
                .parent(&data.main)
                .build(&mut data.quit)?;

            Ok(())
        }
    }

    impl PartialUi for ActionsUi {
        fn build_partial<W: Into<nwg::ControlHandle>>(
            data: &mut ActionsUi,
            parent: Option<W>,
        ) -> Result<(), nwg::NwgError> {
            let parent = parent.unwrap().into();

            nwg::Button::builder()
                .text("Preview")
                .parent(&parent)
                .build(&mut data.preview_button)?;

            nwg::Button::builder()
                .text("Write to file")
                .parent(&parent)
                .build(&mut data.write_button)?;

            nwg::FlexboxLayout::builder()
                .parent(&parent)
                .flex_direction(style::FlexDirection::Row)
                .justify_content(style::JustifyContent::FlexEnd)
                .child(&data.preview_button)
                .child_size(Size {
                    width: Dimension::Points(96.0),
                    height: Dimension::Points(28.0),
                })
                .child(&data.write_button)
                .child_size(Size {
                    width: Dimension::Points(96.0),
                    height: Dimension::Points(28.0),
                })
                .build(&mut data.layout)?;

            Ok(())
        }
    }

    impl PartialUi for PreviewUi {
        fn build_partial<W: Into<nwg::ControlHandle>>(
            data: &mut PreviewUi,
            parent: Option<W>,
        ) -> Result<(), nwg::NwgError> {
            let parent = parent.unwrap().into();

            nwg::Window::builder()
                .flags(nwg::WindowFlags::WINDOW | nwg::WindowFlags::RESIZABLE)
                .size((550, 650))
                .title("Show Content")
                .parent(Some(&parent))
                .build(&mut data.window)?;

            nwg::TextBox::builder()
                .text("")
                .readonly(true)
                .parent(&data.window)
                .build(&mut data.preview)?;

            nwg::FlexboxLayout::builder()
                .parent(&data.window)
                .flex_direction(style::FlexDirection::Column)
                .child(&data.preview)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Percent(1.0),
                })
                .build(&mut data.layout)?;

            Ok(())
        }
    }

    impl PartialUi for Systray {
        fn build_partial<W: Into<nwg::ControlHandle>>(
            data: &mut Systray,
            parent: Option<W>,
        ) -> Result<(), nwg::NwgError> {
            let parent = parent.unwrap().into();

            nwg::Icon::builder()
                .source_bin(Some(ICON_DATA))
                .build(&mut data.icon)?;

            nwg::TrayNotification::builder()
                .parent(&parent)
                .icon(Some(&data.icon))
                .tip(Some("WSL2 IP Writer"))
                .build(&mut data.tray)?;

            nwg::Menu::builder()
                .popup(true)
                .parent(&parent)
                .build(&mut data.tray_menu)?;

            nwg::MenuItem::builder()
                .text("Write")
                .parent(&data.tray_menu)
                .build(&mut data.tray_run)?;

            nwg::MenuItem::builder()
                .text("Open")
                .parent(&data.tray_menu)
                .build(&mut data.tray_open)?;

            nwg::MenuSeparator::builder()
                .parent(&data.tray_menu)
                .build(&mut data.tray_sep)?;

            nwg::MenuItem::builder()
                .text("Exit")
                .parent(&data.tray_menu)
                .build(&mut data.tray_exit)?;

            Ok(())
        }
    }

    impl PartialUi for NamesUi {
        fn build_partial<W: Into<nwg::ControlHandle>>(
            data: &mut NamesUi,
            parent: Option<W>,
        ) -> Result<(), nwg::NwgError> {
            let parent = parent.unwrap().into();

            nwg::Frame::builder()
                .parent(&parent)
                .flags(nwg::FrameFlags::VISIBLE)
                .build(&mut data.list_frame)?;

            nwg::ListBox::builder()
                .parent(&data.list_frame)
                .collection(vec![])
                .build(&mut data.names_list)?;

            nwg::Button::builder()
                .text("Remove")
                .parent(&data.list_frame)
                .build(&mut data.names_remove)?;

            nwg::FlexboxLayout::builder()
                .flex_direction(style::FlexDirection::Row)
                .parent(&data.list_frame)
                .child(&data.names_list)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Auto,
                })
                .child(&data.names_remove)
                .child_size(Size {
                    width: Dimension::Points(96.0),
                    height: Dimension::Points(28.0),
                })
                .build(&mut data.list_row)?;

            nwg::Frame::builder()
                .parent(&parent)
                .flags(nwg::FrameFlags::VISIBLE)
                .build(&mut data.input_frame)?;

            nwg::TextInput::builder()
                .text("")
                .parent(&data.input_frame)
                .placeholder_text(Some("Enter a domain"))
                .build(&mut data.names_input)?;

            nwg::Button::builder()
                .text("Add")
                .parent(&data.input_frame)
                .build(&mut data.names_add)?;

            nwg::FlexboxLayout::builder()
                .flex_direction(style::FlexDirection::Row)
                .parent(&data.input_frame)
                .child(&data.names_input)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Points(28.0),
                })
                .child(&data.names_add)
                .child_size(Size {
                    width: Dimension::Points(96.0),
                    height: Dimension::Points(28.0),
                })
                .build(&mut data.input_row)?;

            nwg::FlexboxLayout::builder()
                .parent(&parent)
                .auto_spacing(None)
                .flex_direction(style::FlexDirection::Column)
                .child(&data.input_frame)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Points(48.0),
                })
                .child(&data.list_frame)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Percent(1.0),
                })
                .build(&mut data.layout)?;

            Ok(())
        }
    }

    impl PartialUi for Options {
        fn build_partial<W: Into<nwg::ControlHandle>>(
            data: &mut Options,
            parent: Option<W>,
        ) -> Result<(), nwg::NwgError> {
            let parent = parent.unwrap().into();

            nwg::Label::builder()
                .text("Path to hosts file")
                .parent(&parent)
                .build(&mut data.hosts_path_label)?;

            nwg::Frame::builder()
                .parent(&parent)
                .flags(nwg::FrameFlags::VISIBLE)
                .build(&mut data.hosts_path_row_frame)?;

            nwg::Button::builder()
                .text("Select file")
                .parent(&data.hosts_path_row_frame)
                .build(&mut data.hosts_path_file_button)?;

            nwg::TextInput::builder()
                .text("")
                .readonly(true)
                .parent(&data.hosts_path_row_frame)
                .build(&mut data.hosts_path_input)?;

            nwg::Button::builder()
                .text("View file")
                .parent(&data.hosts_path_row_frame)
                .build(&mut data.view_hosts_button)?;

            // file select row
            nwg::FlexboxLayout::builder()
                .parent(&data.hosts_path_row_frame)
                .flex_direction(style::FlexDirection::Row)
                .auto_spacing(Some(2))
                .child(&data.hosts_path_input)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Points(28.0),
                })
                .child(&data.hosts_path_file_button)
                .child_size(Size {
                    width: Dimension::Points(128.0),
                    height: Dimension::Points(28.0),
                })
                .child(&data.view_hosts_button)
                .child_size(Size {
                    width: Dimension::Points(128.0),
                    height: Dimension::Points(28.0),
                })
                .build(&mut data.hosts_path_row)?;

            nwg::Frame::builder()
                .parent(&parent)
                .flags(nwg::FrameFlags::VISIBLE)
                .build(&mut data.names_ui_frame)?;

            NamesUi::build_partial(&mut data.names_ui, Some(&data.names_ui_frame))?;

            nwg::FlexboxLayout::builder()
                .parent(&parent)
                .flex_direction(style::FlexDirection::Column)
                .align_items(style::AlignItems::Baseline)
                .auto_spacing(None)
                .child(&data.hosts_path_label)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Points(28.0),
                })
                .child(&data.hosts_path_row_frame)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Points(44.0),
                })
                .child(&data.names_ui_frame)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Percent(1.0),
                })
                .border(Rect {
                    start: Dimension::Points(8.0),
                    end: Dimension::Points(8.0),
                    bottom: Dimension::Points(8.0),
                    top: Dimension::Points(8.0),
                })
                .build(&mut data.layout)?;
            Ok(())
        }
    }

    impl NativeUi<MainUi> for Main {
        fn build_ui(mut data: Main) -> Result<MainUi, nwg::NwgError> {
            nwg::Icon::builder()
                .source_bin(Some(ICON_DATA))
                .build(&mut data.icon)?;

            nwg::Window::builder()
                .icon(Some(&data.icon))
                .flags(nwg::WindowFlags::MAIN_WINDOW)
                .size((500, 400))
                .position((500, 200))
                .title("WSL2 IP Writer")
                .parent(Some(&data.window))
                .build(&mut data.window)?;

            MenuUi::build_partial(&mut data.menu_ui, Some(&data.window))?;

            nwg::FileDialog::builder()
                .title("Select hosts file")
                .action(nwg::FileDialogAction::Open)
                .build(&mut data.hosts_file_dialog)?;

            nwg::Frame::builder()
                .parent(&data.window)
                .flags(nwg::FrameFlags::VISIBLE)
                .build(&mut data.options_frame)?;

            nwg::Frame::builder()
                .parent(&data.window)
                .flags(nwg::FrameFlags::VISIBLE)
                .build(&mut data.actions_frame)?;

            nwg::StatusBar::builder()
                .parent(&data.window)
                .text("")
                .build(&mut data.status)?;

            Systray::build_partial(&mut data.tray, Some(&data.window))?;
            Options::build_partial(&mut data.options, Some(&data.options_frame))?;
            ActionsUi::build_partial(&mut data.actions_ui, Some(&data.actions_frame))?;
            PreviewUi::build_partial(&mut data.preview_ui, Some(&data.window))?;

            nwg::FlexboxLayout::builder()
                .parent(&data.window)
                .flex_direction(style::FlexDirection::Column)
                .child(&data.options_frame)
                .auto_spacing(None)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Percent(1.0),
                })
                .child(&data.actions_frame)
                .child_size(Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Points(48.0),
                })
                .child_align_self(style::AlignSelf::Baseline)
                .child_margin(Rect {
                    start: Dimension::Points(0.0),
                    end: Dimension::Points(0.0),
                    top: Dimension::Points(8.0),
                    bottom: Dimension::Points(32.0),
                })
                .build(&mut data.layout)?;

            let ui = MainUi {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };

            let handle = &ui.window.handle;
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, evt_data, handle| {
                if let Some(evt_ui) = evt_ui.upgrade() {
                    evt_ui.options.process_event(evt, &evt_data, handle);
                    match evt {
                        Event::OnContextMenu => {
                            if &handle == &evt_ui.tray.tray {
                                Main::show_menu(&evt_ui);
                            }
                        }
                        Event::OnButtonClick => {
                            if &handle == &evt_ui.options.hosts_path_file_button {
                                Main::select_file(&evt_ui);
                            } else if &handle == &evt_ui.options.view_hosts_button {
                                Main::show_hosts_file(&evt_ui);
                            } else if &handle == &evt_ui.actions_ui.write_button {
                                Main::write(&evt_ui);
                            } else if &handle == &evt_ui.actions_ui.preview_button {
                                Main::show_preview(&evt_ui);
                            } else if &handle == &evt_ui.options.names_ui.names_add {
                                Main::add_name(&evt_ui)
                            } else if &handle == &evt_ui.options.names_ui.names_remove {
                                Main::remove_name(&evt_ui)
                            }
                        }
                        Event::OnMenuItemSelected => {
                            if &handle == &evt_ui.tray.tray_run {
                                Main::write(&evt_ui);
                            } else if &handle == &evt_ui.tray.tray_open {
                                Main::open(&evt_ui);
                            } else if &handle == &evt_ui.tray.tray_exit {
                                Main::on_exit(&evt_ui);
                            } else if &handle == &evt_ui.menu_ui.save {
                                Main::save_config(&evt_ui);
                            } else if &handle == &evt_ui.menu_ui.quit {
                                Main::on_exit(&evt_ui);
                            }
                        }
                        Event::OnInit => {
                            if &handle == &evt_ui.window.handle {
                                Main::on_init(&evt_ui);
                            }
                        }
                        _ => {}
                    }
                }
            };

            let handler = nwg::full_bind_event_handler(handle, handle_events);
            ui.default_handler.borrow_mut().push(handler);

            Ok(ui)
        }
    }

    impl Drop for MainUi {
        fn drop(&mut self) {
            let mut handlers = self.default_handler.borrow_mut();
            for handler in handlers.drain(0..) {
                nwg::unbind_event_handler(&handler);
            }
        }
    }

    impl std::ops::Deref for MainUi {
        type Target = Main;

        fn deref(&self) -> &Main {
            &self.inner
        }
    }
}

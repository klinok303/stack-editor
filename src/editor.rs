use crossterm::event::{read, Event, KeyEvent, KeyEventKind, KeyCode, KeyModifiers};
use std::{
    env, io::Error, panic::{set_hook, take_hook}
};
mod annotatedstring;
mod command;
mod uicomponents;
mod documentstatus;
mod line;
mod terminal;
mod prelude;
use prelude::*;

use annotatedstring::{AnnotatedString, AnnotationType};
use uicomponents::{CommandBar,MessageBar,View, StatusBar, UIComponent};
use documentstatus::DocumentStatus;
use line::Line;
use terminal::Terminal;
use self::command::Bindings;

use stack_editor_macros::insert_into_map;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
enum PromptType {
    Save,
    Find,
    #[default]
    None,
}


#[derive(Default)]
pub struct Editor {
    should_quit: bool,
    view: View,
    bindings: Bindings,
    status_bar: StatusBar,
    message_bar: MessageBar,
    terminal_size: Size,
    title: String,
    quit_times: u8,
    command_bar: CommandBar,
    prompt_type: PromptType,
}

impl Editor {
    pub fn new() -> Result<Self, Error> {
        let current_hook = take_hook();
        set_hook(Box::new(move |panic_info| {
            let _ = Terminal::terminate();
            current_hook(panic_info);
        }));
        Terminal::initialize()?;

        let mut editor = Self::default();
        let size = Terminal::size().unwrap_or_default();
        
        insert_into_map!(&mut editor.bindings, {
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => "save",
            (KeyCode::Char('q'), KeyModifiers::CONTROL) => "quit",
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => "find",
            (KeyCode::Up, KeyModifiers::NONE) => "move_up",
            (KeyCode::Down, KeyModifiers::NONE) => "move_down",
            (KeyCode::Left, KeyModifiers::NONE) => "move_left",
            (KeyCode::Right, KeyModifiers::NONE) => "move_right",
            (KeyCode::PageUp, KeyModifiers::NONE) => "page_up",
            (KeyCode::PageDown, KeyModifiers::NONE) => "page_down",
            (KeyCode::Home, KeyModifiers::NONE) => "to_start_of_the_line",
            (KeyCode::End, KeyModifiers::NONE) => "to_end_of_the_file",
            (KeyCode::Tab, KeyModifiers::NONE) => "tab",
            (KeyCode::Esc, KeyModifiers::NONE) => "dismiss",
            (KeyCode::Enter, KeyModifiers::NONE) => "insert_newline",
            (KeyCode::Backspace, KeyModifiers::NONE) => "delete_backward",
            (KeyCode::Delete, KeyModifiers::NONE) => "delete",
        });

        editor.resize(size);
        editor
            .message_bar
            .update_message("HELP: Ctrl-F = find | Ctrl-S = save | Ctrl-Q = quit");

        let args: Vec<String> = env::args().collect();
        if let Some(file_name) = args.get(1) {
            debug_assert!(!file_name.is_empty());
            if editor.view.load(file_name).is_err() {
                editor
                    .message_bar
                    .update_message(&format!("ERR: Could not open file: {file_name}"));
            }
        }

        editor.refresh_status();
        Ok(editor)
    }

    fn resize(&mut self, size: Size) {
        self.terminal_size = size;
        self.view.resize(Size {
            height: size.height.saturating_sub(2),
            width: size.width,
        });
        let bar_size = Size {
            height: 1,
            width: size.width,
        };
        self.message_bar.resize(bar_size);
        self.status_bar.resize(bar_size);
        self.command_bar.resize(bar_size);
    }

    fn refresh_status(&mut self) {
        let status = self.view.get_status();
        let title = format!("{} - {NAME}", status.file_name);
        self.status_bar.update_status(status);

        if title != self.title && matches!(Terminal::set_title(&title), Ok(())) {
            self.title = title;
        }
    }

    pub fn run(&mut self) {
        loop {
            self.refresh_screen();
            if self.should_quit {
                break;
            }
            match read() {
                Ok(event) => self.evaluate_event(event),
                Err(err) => {
                    #[cfg(debug_assertions)]
                    {
                        panic!("Could not read event: {err:?}");
                    }

                    #[cfg(not(debug_assertions))]
                    {
                        let _ = err;
                    }
                }
            }
            let status = self.view.get_status();
            self.status_bar.update_status(status);
        }
    }

    fn evaluate_event(&mut self, event: Event) {
        let should_process = match &event {
            Event::Key(KeyEvent { kind, .. }) => kind == &KeyEventKind::Press,
            Event::Resize(_, _) => true,
            _ => false,
        };

        if !should_process {
            return;
        }

        match event {
            Event::Key(key_event) => {
                let KeyEvent { code, modifiers, .. } = key_event;
                let command = self.bindings.event_check(key_event).unwrap_or_default();

                match code {
                    KeyCode::Char(c) => {
                        if modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT {
                            if self.prompt_type != PromptType::None {
                                self.command_bar.append_char(c);
                                self.command_bar.redraw();

                                if self.prompt_type == PromptType::Find {
                                    let query = self.command_bar.value();
                                    self.view.search(&query);
                                }
                            } else {
                                self.view.insert_char(c);
                            }
                        } else {
                            self.process_command(command);
                        }
                    }
                    _ => {
                        self.process_command(command);
                    }
                }
            }
            Event::Resize(width, height) => {
                self.resize(Size {
                    width: width as usize,
                    height: height as usize,
                });
            }
            _ => {}
        }
    }

    fn process_command(&mut self, command: String) {
        match command.as_str() {
            "quit" => self.handle_quit(),
            _ => self.reset_quit_times(),
        }

        match command.as_str() {
            "save" => self.handle_save(),
            "quit" => {}, // Already handled above
            
            // Search/replace
            "find" => self.show_prompt(PromptType::Find),

            // Navigation
            "move_up" => {
                if self.prompt_type == PromptType::Find {
                    self.view.search_prev()
                } else {
                    self.view.move_up(1)
                }
            }
            "move_down" => {
                if self.prompt_type == PromptType::Find {
                    self.view.search_next();
                } else {
                    self.view.move_down(1)
                }
            },
            "move_left" => {
                if self.prompt_type == PromptType::Find {
                    self.view.search_prev()
                } else {
                    self.view.move_left()
                }
            },
            "move_right" => {
                if self.prompt_type == PromptType::Find {
                    self.view.search_next();
                } else {
                    self.view.move_right()
                }
            },
            "page_up" => self.view.move_up(self.view.get_size().height.saturating_sub(1)),
            "page_down" => self.view.move_down(self.view.get_size().height.saturating_sub(1)),
            "to_start_of_the_line" => self.view.move_to_start_of_line(),
            "to_end_of_the_file" => self.view.move_to_end_of_line(),
            
            // Editing
            "delete" => {
                if self.prompt_type == PromptType::Find {
                    self.command_bar.delete();
                    let query = self.command_bar.value();
                    self.view.search(&query);
                } else {
                    self.view.delete()
                }
            }
            "delete_backward" => {
                if self.prompt_type != PromptType::None {
                    self.command_bar.delete_last();
                    self.command_bar.redraw();
                    if self.prompt_type == PromptType::Find {
                        let query = self.command_bar.value();
                        self.view.search(&query);
                    }
                } else {
                    self.view.delete_backward();
                }
            }
            "tab" => self.view.insert_char('\t'),
            
            // Prompts
            "dismiss" => self.dismiss_prompt(),
            "insert_newline" => self.handle_enter_press(),
            
            _ => {}
        }

        // Handle view updates
        self.handle_view_updates(command);
    }

    fn handle_enter_press(&mut self) {
        if self.prompt_type != PromptType::None {
            let value = self.command_bar.value().clone();
            
            match self.prompt_type {
                PromptType::Save => self.save(Some(&value)),
                PromptType::Find => self.view.exit_search(),
                PromptType::None => unreachable!(),
            }
            
            self.command_bar.clear_value();
            self.prompt_type = PromptType::None;
            self.message_bar.set_needs_redraw(true);
            self.view.set_needs_redraw(true);
            self.status_bar.set_needs_redraw(true);
        } else {
            self.view.insert_newline();
        }
    }

    fn handle_view_updates(&mut self, command: String) {
        if matches!(
            command.as_str(),
            "move_up" | "move_down" | 
            "move_left" | "move_right" |
            "page_up" | "page_down" |
            "to_start_of_the_line" | "to_end_of_the_file"
        ) {
            self.view.scroll_text_location_into_view();
        }
    }

    fn show_prompt(&mut self, prompt_type: PromptType) {
        match prompt_type {
            PromptType::Save => self.command_bar.set_prompt("Save as: "),
            PromptType::Find => {
                self.command_bar
                    .set_prompt("Search (Esc to cancel, Arrows to navigate): ");
                self.view.enter_search();
            }
            _ => return,
        }
        
        self.command_bar.resize(Size {
            height: 1,
            width: self.terminal_size.width,
        });
        self.prompt_type = prompt_type;
    }

    fn dismiss_prompt(&mut self) {
        match self.prompt_type {
            PromptType::Find => self.view.dismiss_search(),
            PromptType::Save => {},
            PromptType::None => self.handle_quit(),
        }
        self.command_bar.clear_value();
        self.prompt_type = PromptType::None;
        self.message_bar.set_needs_redraw(true);
        self.view.set_needs_redraw(true);
        self.status_bar.set_needs_redraw(true);
    }

    fn update_message(&mut self, new_message: &str) {
        self.message_bar.update_message(new_message);
    }

    fn handle_save(&mut self) {
        if self.view.is_file_loaded() {
            self.save(None);
        } else {
            self.show_prompt(PromptType::Save);
        }
    }

    fn save(&mut self, file_name: Option<&str>) {
        let result = if let Some(name) = file_name {
            self.view.save_as(name)
        } else {
            self.view.save()
        };
        if result.is_ok() {
            self.update_message("File saved successfully.");
        } else {
            self.update_message("Error writing file!");
        }
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn handle_quit(&mut self) {
        if !self.view.get_status().is_modified || self.quit_times + 1 == QUIT_TIMES {
            self.should_quit = true;
        } else if self.view.get_status().is_modified {
            self.update_message(&format!(
                "WARNING! File has unsaved changes. Press Ctrl-Q {} more times to quit.",
                QUIT_TIMES - self.quit_times - 1
            ));

            self.quit_times += 1;
        }
    }

    fn reset_quit_times(&mut self) {
        if self.quit_times > 0 {
            self.quit_times = 0;
            self.message_bar.update_message("");
        }
    }

    fn refresh_screen(&mut self) {
        if self.terminal_size.height < 3 || self.terminal_size.width == 0 {
            return;
        }

        let _ = Terminal::hide_caret();
        let mut row = 0;

        let content_height = self.terminal_size.height.saturating_sub(2);
        self.view.resize(Size {
            width: self.terminal_size.width,
            height: content_height,
        });
        self.view.render(row);
        row += content_height;

        // Рендер статус-бара
        self.status_bar.resize(Size {
            width: self.terminal_size.width,
            height: 1,
        });
        self.status_bar.render(row);
        row += 1;

        // Рендер командной строки или сообщений
        if self.prompt_type != PromptType::None {
            self.command_bar.resize(Size {
                width: self.terminal_size.width,
                height: 1,
            });
            self.command_bar.render(row);
        } else {
            self.message_bar.resize(Size {
                width: self.terminal_size.width,
                height: 1,
            });
            self.message_bar.render(row);
        }

        // 3. Корректное позиционирование каретки
        let caret_pos = if self.prompt_type != PromptType::None {
            Position {
                row: self.terminal_size.height.saturating_sub(1),
                col: self.command_bar.caret_position_col(),
            }
        } else {
            self.view.caret_position()
        };

        let _ = Terminal::move_caret_to(caret_pos);
        let _ = Terminal::show_caret();
        let _ = Terminal::execute();
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        let _ = Terminal::terminate();
        if self.should_quit {
            let _ = Terminal::print("Goodbye.\r\n");
        }
    }
}

use crossterm::{
    event::{read, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    terminal::CompletedFrame,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{
    io::{self, stdout, Error, ErrorKind, Stdout},
    process::{exit, Output},
};

use crate::tocker::{Message, Moment, Select, TargetType, Tocker};

const INITIAL_COMMANDS: &str =
    "Available commands: \n press 'i' = image, 'c' = container, 'v' = volume.";
const TARGET_COMMANDS: &str = "Available commands: \n press 'space' = select, 'enter' = confirm";

struct ContentItem {
    text: String,
    selected: bool,
}

struct Scroller {
    // offset: usize,
    cursor: usize,
}

pub struct AppState {
    content: Vec<ContentItem>,
    commands: String,
    moment: Moment,
    scroll: Scroller,
}

pub struct Tui {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    tocker: Tocker,
    state: AppState,
}

impl Tui {
    pub fn new() -> Result<Tui, Error> {
        //clear screen
        enable_raw_mode()?;
        execute!(stdout(), EnterAlternateScreen)?;

        // backend
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend)?;

        // tocker services
        let tocker = Tocker::new();

        // initial state
        let initial_commands = String::from(INITIAL_COMMANDS);
        let initial_content: Vec<ContentItem> = vec![];
        let initial_moment = Moment::KIND;

        // initial scroll
        let initial_scroll = Scroller {
            // offset: 0,
            cursor: 0,
        };

        // instantiate the tui program
        Ok(Tui {
            terminal,
            tocker,
            state: AppState {
                content: initial_content,
                commands: initial_commands,
                moment: initial_moment,
                scroll: initial_scroll,
            },
        })
    }

    pub fn draw_ui(&mut self) -> io::Result<CompletedFrame> {
        self.terminal.draw(|f| {
            // scaffold ui
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([Constraint::Percentage(90), Constraint::Percentage(10)].as_ref())
                .split(f.size());
            // content
            let items: Vec<ListItem> = self
                .state
                .content
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    ListItem::new(item.text.as_ref()).style(
                        match index == self.state.scroll.cursor {
                            true => match self.state.scroll.cursor == 0 {
                                true => Style::default(),
                                false => Style::default().bg(Color::Cyan).fg(Color::Black),
                            },
                            false => match item.selected {
                                true => Style::default().bg(Color::Gray).fg(Color::Black),
                                false => Style::default(),
                            },
                        },
                    )
                })
                .collect();
            f.render_widget(
                List::new(items).block(Block::default().borders(Borders::ALL)),
                chunks[0],
            );
            // display available commands
            let p = Paragraph::new(self.state.commands.as_ref())
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::White).bg(Color::Black))
                .alignment(Alignment::Left);
            f.render_widget(p, chunks[1]);
        })
    }

    fn extract_target_string(&mut self) -> String {
        let mut string = String::from("");
        let mut id_column_index: usize = 0;
        self.state
            .content
            .iter()
            .enumerate()
            .for_each(|(index, item)| {
                if index == 0 {
                    item.text
                        .split_whitespace()
                        .enumerate()
                        .for_each(|(index, content)| {
                            if content.contains("ID") {
                                id_column_index = index - 1; //"container id", "image id"
                            }
                        });
                }
                if index > 0 && item.selected {
                    item.text
                        .split_whitespace()
                        .enumerate()
                        .for_each(|(index, content)| {
                            if index == id_column_index {
                                string.push_str(" ");
                                string.push_str(content.as_ref());
                            }
                        });
                }
            });
        String::from(string.trim())
    }

    fn check_select(&mut self, key_event: KeyEvent) -> Result<&Select, Error> {
        self.tocker.check_select(key_event)
    }

    fn update_commands_target(&mut self) -> Result<(), Error> {
        self.state.commands = String::from(TARGET_COMMANDS);
        self.draw_ui().ok();
        Ok(())
    }

    fn update_available_commands(&mut self, first_key: &KeyEvent) -> Result<(), Error> {
        self.state.commands = self.tocker.get_available_commands(first_key).cloned()?;
        self.draw_ui().ok();
        Ok(())
    }

    fn check_combination(
        &mut self,
        first: &KeyEvent,
        second: &KeyEvent,
    ) -> Result<&TargetType, Error> {
        self.tocker.check_for_target(first, second)
    }

    fn execute_cmd(
        &mut self,
        first: &KeyEvent,
        second: &KeyEvent,
        target: &String,
    ) -> Result<Output, Error> {
        self.tocker.exec_cmd(first, second, target)
    }

    fn update_moment(&mut self, new_moment: Moment) {
        self.state.moment = new_moment;
    }

    fn quit_tocker(&mut self) -> () {
        disable_raw_mode().expect("Error in disabling raw mode");
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen,)
            .expect("Error in leaving alternate screen");
        self.terminal
            .clear()
            .expect("Error in cleaning back the terminal");
        self.terminal
            .show_cursor()
            .expect("Error in showing back the cursor");
        self.terminal
            .set_cursor(0, 0)
            .expect("Error in setting cursor at the top");
        exit(0)
    }

    fn clean(&mut self) -> Result<(), Error> {
        self.state.content = vec![];
        self.draw_ui()?;
        Ok(())
    }

    fn help(&mut self) -> Result<(), Error> {
        match self.state.moment {
            Moment::KIND => {
                self.state.commands = self.tocker.get_help_commands().clone();
                self.draw_ui()?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn cancel(&mut self) -> Error {
        self.go_to_first();
        Error::new(ErrorKind::Interrupted, "User canceled the action")
    }

    fn wrong(&mut self) -> Error {
        Error::new(ErrorKind::InvalidInput, "Press only the available keys")
    }

    fn next_action(&mut self, message: Message) -> Result<(), Error> {
        match message {
            Message::HELP => self.help(),
            Message::CLEAN => self.clean(),
            Message::CANCEL => Err(self.cancel()),
            Message::QUIT => Ok(self.quit_tocker()),
            Message::WRONG => Err(self.wrong()),
            Message::OK => Ok(()), //next key or check_combination
        }
    }

    fn check_key(&mut self, key_event: &KeyEvent) -> Result<Message, Error> {
        self.tocker.check_keybinding(key_event, &self.state.moment)
    }

    fn extract_key_event(&mut self) -> Result<KeyEvent, Error> {
        self.tocker.extract_key_event(read()?)
    }

    fn go_to_first(&mut self) {
        let initial_commands = String::from(INITIAL_COMMANDS);
        self.state.commands = initial_commands;
        self.state.scroll.cursor = 0;
        self.draw_ui().ok();
        self.update_moment(Moment::KIND);
    }

    fn go_to_second(&mut self, first: &KeyEvent) -> Result<(), Error> {
        self.update_available_commands(first)?;
        self.update_moment(Moment::COMMAND);
        Ok(())
    }

    fn get_second(&mut self) -> Result<KeyEvent, Error> {
        loop {
            let second = self.extract_key_event()?;
            let msg_answer = self.check_key(&second)?;
            self.next_action(msg_answer)?;
            break Ok(second);
        }
    }

    fn get_first(&mut self) -> Result<KeyEvent, Error> {
        let first = self.extract_key_event()?;
        let msg_answer = self.check_key(&first)?;
        self.next_action(msg_answer)?;
        self.go_to_second(&first)?;
        Ok(first)
    }

    fn add_cursor(&mut self) {
        self.state.scroll.cursor += 1;
        if self.state.scroll.cursor >= self.state.content.len() {
            self.state.scroll.cursor = 1;
        }
    }

    fn sub_cursor(&mut self) {
        self.state.scroll.cursor -= 1;
        if self.state.scroll.cursor <= 0 {
            self.state.scroll.cursor = self.state.content.len() - 1;
        }
    }

    fn looping(&mut self) -> Result<(), Error> {
        // collect key presses combo
        let first = self.get_first()?;
        let second = self.get_second()?;

        // check target type
        let target_type = self.check_combination(&first, &second)?;
        match target_type {
            TargetType::SELECT => loop {
                self.update_commands_target()?;
                let key_event = self.extract_key_event()?;
                let select = self.check_select(key_event)?;
                match select {
                    Select::UP => self.add_cursor(),
                    Select::DOWN => self.sub_cursor(),
                    Select::SELECT => match self.state.content.get_mut(self.state.scroll.cursor) {
                        Some(item) => item.selected = !item.selected,
                        None => {}
                    },
                    Select::CANCEL => {
                        self.go_to_first();
                        break;
                    }
                    Select::CONFIRM => {
                        break;
                    }
                }
            },
            _ => {}
        }
        let target_string = self.extract_target_string();

        let output = String::from_utf8(
            self.execute_cmd(&first, &second, &target_string)
                .unwrap()
                .stdout,
        )
        .unwrap();

        self.state.content = output
            .lines()
            .map(|line| {
                let text = String::from(line);
                let selected = false;
                ContentItem { text, selected }
            })
            .collect();

        self.draw_ui().ok();

        self.go_to_first();

        Ok(())
    }

    pub fn start_loop(&mut self) -> () {
        loop {
            if let Err(err) = self.looping() {
                self.state.content.push(ContentItem {
                    text: String::from(err.to_string()),
                    selected: false,
                });
            }
        }
    }
}

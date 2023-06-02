use std::{
    collections::HashMap,
    ffi::OsString,
    io::{Error, ErrorKind},
    process::{exit, Command, ExitStatus, Output, Stdio},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug)]
pub enum Moment {
    KIND,
    COMMAND,
    TARGET,
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum DockerKind {
    Image,
    Container,
    Volume,
}

impl From<&DockerKind> for OsString {
    fn from(value: &DockerKind) -> Self {
        match value {
            DockerKind::Image => OsString::from("image"),
            DockerKind::Container => OsString::from("container"),
            DockerKind::Volume => OsString::from("volume"),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub enum DockerCommand {
    LS,
    RM,
    TAG,
    STOP,
}

impl From<&DockerCommand> for OsString {
    fn from(value: &DockerCommand) -> Self {
        match value {
            DockerCommand::LS => OsString::from("ls"),
            DockerCommand::RM => OsString::from("rm"),
            DockerCommand::TAG => OsString::from("tag"),
            DockerCommand::STOP => OsString::from("stop"),
        }
    }
}

#[derive(Debug)]
struct AllowedCommands {
    mapping: HashMap<DockerKind, Vec<DockerCommand>>,
    legenda: HashMap<DockerKind, String>,
}

#[derive(Debug)]
pub enum TargetType {
    INPUT,
    SELECT,
    EMPTY,
}

#[derive(Debug)]
pub struct DockerPrompt<'a> {
    pub kind: &'a DockerKind,
    pub command: &'a DockerCommand,
    pub target: &'a String,
}

#[derive(Debug)]
pub enum GeneralCommand {
    QUIT,
    CANCEL,
    HELP,
    CLEAN,
    // BUILD,
}

#[derive(Debug)]
pub enum Message {
    OK,
    WRONG,
    QUIT,
    CANCEL,
    HELP,
    CLEAN,
}

#[derive(Debug)]
pub enum Select {
    UP,
    DOWN,
    SELECT,
    CONFIRM,
    CANCEL,
}

pub struct Tocker {
    kind_keybindings: HashMap<KeyEvent, DockerKind>,
    command_keybindings: HashMap<KeyEvent, DockerCommand>,
    general_keybindings: HashMap<KeyEvent, GeneralCommand>,
    select_keybindings: HashMap<KeyEvent, Select>,
    target_mapping: HashMap<DockerCommand, TargetType>,
    allowed_commands: AllowedCommands,
    help_string: String,
}

impl Tocker {
    pub fn new() -> Tocker {
        let status = Command::new("docker")
            .arg("info")
            .stdout(Stdio::null())
            .status()
            .expect("Failed to contact deamon");
        if !ExitStatus::success(&status) {
            exit(1);
        }

        let kind_keybindings = HashMap::from([
            (
                KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
                DockerKind::Image,
            ),
            (
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
                DockerKind::Container,
            ),
            (
                KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE),
                DockerKind::Volume,
            ),
        ]);
        let command_keybindings = HashMap::from([
            (
                KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE),
                DockerCommand::LS,
            ),
            (
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE),
                DockerCommand::RM,
            ),
            (
                KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE),
                DockerCommand::STOP,
            ),
            (
                KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE),
                DockerCommand::TAG,
            ),
        ]);
        let general_keybindings = HashMap::from([
            (
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                GeneralCommand::CANCEL,
            ),
            (
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                GeneralCommand::CANCEL,
            ),
            (
                KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
                GeneralCommand::QUIT,
            ),
            (
                KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL),
                GeneralCommand::HELP,
            ),
            (
                KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
                GeneralCommand::CLEAN,
            ),
            // (
            //     KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
            //     GeneralCommand::BUILD,
            // ),
        ]);

        let help_string = String::from(
            "[c/i/v] = container/image/volume; \n [ctrl+q] = quit; [ctrl+c] = cancel action; [ctrl+l] = clear content; [ctrl+b] build image from path",
        );

        let mapping = HashMap::from([
            (
                DockerKind::Image,
                vec![DockerCommand::LS, DockerCommand::RM, DockerCommand::TAG],
            ),
            (
                DockerKind::Container,
                vec![DockerCommand::LS, DockerCommand::RM, DockerCommand::STOP],
            ),
            (
                DockerKind::Volume,
                vec![DockerCommand::LS, DockerCommand::RM],
            ),
        ]);
        let legenda = HashMap::from([
            (
                DockerKind::Image,
                String::from("Available commands for image: \n l = ls, r = rm, t = tag"),
            ),
            (
                DockerKind::Container,
                String::from("Available commands for container: \n l = ls, r = rm, s = stop"),
            ),
            (
                DockerKind::Volume,
                String::from("Available commands for volume: \n l = ls, r = rm"),
            ),
        ]);

        let allowed_commands = AllowedCommands { mapping, legenda };

        let target_mapping = HashMap::from([
            (DockerCommand::RM, TargetType::SELECT),
            (DockerCommand::STOP, TargetType::SELECT),
            (DockerCommand::LS, TargetType::EMPTY),
            (DockerCommand::TAG, TargetType::INPUT),
        ]);

        let select_keybindings = HashMap::from([
            (KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), Select::UP),
            (
                KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
                Select::UP,
            ),
            (
                KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
                Select::DOWN,
            ),
            (
                KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE),
                Select::DOWN,
            ),
            (
                KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE),
                Select::SELECT,
            ),
            (
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                Select::CONFIRM,
            ),
            (
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                Select::CANCEL,
            ),
        ]);

        Tocker {
            kind_keybindings,
            command_keybindings,
            general_keybindings,
            select_keybindings,
            target_mapping,
            allowed_commands,
            help_string,
        }
    }

    pub fn extract_key_event(&self, e: Event) -> Result<KeyEvent, Error> {
        match e {
            Event::Key(key_event) => Ok(key_event),
            _ => Err(Error::new(ErrorKind::InvalidInput, "Press a valid key")),
        }
    }

    pub fn check_select(&self, event: KeyEvent) -> Result<&Select, Error> {
        self.select_keybindings.get(&event).ok_or(Error::new(
            ErrorKind::InvalidInput,
            "Invalid key input for selection",
        ))
    }

    pub fn check_keybinding(&self, event: &KeyEvent, moment: &Moment) -> Result<Message, Error> {
        match self.general_keybindings.get(event) {
            Some(cmd) => match cmd {
                GeneralCommand::QUIT => Ok(Message::QUIT),
                GeneralCommand::CANCEL => Ok(Message::CANCEL),
                GeneralCommand::HELP => Ok(Message::HELP),
                GeneralCommand::CLEAN => Ok(Message::CLEAN),
                // GeneralCommand::BUILD => Ok(Message::BUILD),
            },
            None => match moment {
                Moment::KIND => match self.kind_keybindings.get(event) {
                    Some(_) => Ok(Message::OK),
                    None => Ok(Message::WRONG),
                },
                Moment::COMMAND => match self.command_keybindings.get(event) {
                    Some(_) => Ok(Message::OK),
                    None => Ok(Message::WRONG),
                },
                Moment::TARGET => Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Input should not be considered as commands",
                )),
            },
        }
    }

    pub fn get_help_commands(&self) -> &String {
        &self.help_string
    }

    pub fn get_available_commands(&self, key_event: &KeyEvent) -> Result<&String, Error> {
        let input_err = Error::new(
            ErrorKind::InvalidInput,
            "Key pressed doesn't have any available commands",
        );
        let Some(kind) = self.kind_keybindings.get(key_event) else { return Err(input_err) };
        let Some(command_string) = self.allowed_commands.legenda.get(kind) else { return Err(input_err) };
        Ok(command_string)
    }

    pub fn check_for_target(
        &self,
        first: &KeyEvent,
        second: &KeyEvent,
    ) -> Result<&TargetType, Error> {
        let input_err = Error::new(ErrorKind::InvalidInput, "Not valid inputs");
        let Some(kind) = self.kind_keybindings.get(first) else { return Err(input_err) };
        let Some(allowed_commands) = self.allowed_commands.mapping.get(kind) else { return Err(input_err) };
        let Some(cmd) = self.command_keybindings.get(second) else { return Err(input_err) };
        if allowed_commands.contains(cmd) {
            let Some(target) = self.target_mapping.get(cmd) else { return Err(input_err) };
            Ok(target)
        } else {
            Err(input_err)
        }
    }

    pub fn exec_cmd(
        &self,
        first: &KeyEvent,
        second: &KeyEvent,
        target: &String,
    ) -> Result<Output, Error> {
        let kind = self
            .kind_keybindings
            .get(first)
            .ok_or(Error::new(ErrorKind::InvalidInput, "Invalid kind input"))?;
        let command = self
            .command_keybindings
            .get(second)
            .ok_or(Error::new(ErrorKind::InvalidInput, "Invalid command input"))?;
        let prompt = DockerPrompt {
            kind,
            command,
            target,
        };
        self.docker_execute_prompt(prompt)
    }

    pub fn docker_execute_prompt(&self, cmd: DockerPrompt) -> Result<Output, Error> {
        Command::new("docker")
            .arg(OsString::from(cmd.kind))
            .arg(OsString::from(cmd.command))
            .arg(OsString::from(cmd.target))
            .output()
    }
}

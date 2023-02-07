extern crate regex;
mod mods;

use chrono::Local;
use ncurses::*;
use std::sync::atomic::Ordering;

use mods::todo::*;
use mods::ui::*;
use mods::utils::*;

const SELECTED_PAIR: i16 = 1;
const UNSELECTED_PAIR: i16 = 2;
const HIGHLIGHT_PAIR: i16 = 3;
const UI_PAIR: i16 = 4;

const USAGE: &str = "Usage: todo [-f | --file <file>] [-h | --help]";
const HELP: &str = r#"ToDors - a simple todo list manager in terminal.
Author: Iurii Kondrakov <deezzir@gmail.com>

    Options:
        -f, --file <file>   The file to use for the todo list.
        -h, --help          Show this help message.

    Controls:
        <k/up>, <j/down>  ~ Move the cursor up
        <K>, <J>          ~ Drag item UP/DOWN
        <g>, <G>          ~ Go to the top/bottom of the list
        <d>               ~ Delete 'Done' element
        <i>               ~ Insert a new 'Todo' element
        <u>               ~ Undo last action
        <r>               ~ Edit current item
        <enter>           ~ Transfer current elemen/Save edited element
        <esc>             ~ Cancel editing
        <tab>             ~ Switch between Switch between 'Todos'/'Dones'
        <q>, <ctrl-c>     ~ Quit
"#;

const FILE_PATH: &str = "TODO";

#[derive(PartialEq)]
enum Mode {
    Edit,
    Normal,
}

fn main() {
    set_sig_handler();
    let file_path: String = get_args();

    ncurses_init();
    let mut mode: Mode = Mode::Normal;
    let mut editing_cursor: usize = 0;
    let mut app: TodoApp = TodoApp::new();
    let mut ui = UI::new();

    app.parse(&file_path);
    while !QUIT.load(Ordering::SeqCst) {
        erase();
        let date = Local::now().format("%Y %a %b %d %H:%M:%S");
        let mut w = 0;
        let mut h = 0;
        getmaxyx(stdscr(), &mut h, &mut w);

        ui.begin(Vec2::new(0, 0), LayoutKind::Vert, Vec2::new(w, h));
        {
            ui.begin_layout(LayoutKind::Horz);
            {
                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label_styled(
                        &format!(
                            "[CONTENT]: ({})todos and ({})dones",
                            app.get_todos_n(),
                            app.get_dones_n()
                        ),
                        UI_PAIR,
                        Some(A_BOLD()),
                    );
                    ui.label_styled(
                        &format!("[MESSAGE]: {}", app.get_message()),
                        UI_PAIR,
                        Some(A_BOLD()),
                    );
                }
                ui.end_layout();

                ui.begin_layout(LayoutKind::Vert);
                {
                    ui.label_styled(&format!("[DATE]: {date}"), UI_PAIR, Some(A_BOLD()));
                    ui.label_styled(&format!("[FILE]: {file_path}"), UI_PAIR, Some(A_BOLD()));
                }
                ui.end_layout();
            }
            ui.end_layout();

            ui.hl();
            ui.br();

            ui.begin_layout(LayoutKind::Horz);
            {
                ui.begin_layout(LayoutKind::Vert);
                {
                    if app.is_in_todo_panel() {
                        ui.label_styled("[TODO]", HIGHLIGHT_PAIR, None);
                    } else {
                        ui.label_styled(" TODO ", UNSELECTED_PAIR, None);
                    }
                    ui.hl();
                    for todo in app.get_todos() {
                        if app.is_cur_todo(todo) {
                            if app.is_in_todo_panel() {
                                if mode == Mode::Edit {
                                    ui.edit_label(
                                        todo.get_text(),
                                        editing_cursor,
                                        "- [ ] ".to_string(),
                                    );
                                } else {
                                    ui.label_styled(
                                        &format!("- [ ] {}", todo.get_text()),
                                        SELECTED_PAIR,
                                        None,
                                    );
                                }
                            } else {
                                ui.label_styled(
                                    &format!("- [ ] {}", todo.get_text()),
                                    UNSELECTED_PAIR,
                                    None,
                                );
                            }
                        } else {
                            ui.label(&format!("- [ ] {}", todo.get_text()));
                        }
                    }
                }
                ui.end_layout();

                ui.begin_layout(LayoutKind::Vert);
                {
                    if app.is_in_done_panel() {
                        ui.label_styled("[DONE]", HIGHLIGHT_PAIR, None);
                    } else {
                        ui.label_styled(" DONE ", UNSELECTED_PAIR, None);
                    }
                    ui.hl();
                    for done in app.get_dones() {
                        if app.is_cur_done(done) {
                            if app.is_in_done_panel() {
                                if mode == Mode::Edit {
                                    ui.edit_label(
                                        done.get_text(),
                                        editing_cursor,
                                        "- [X] ".to_string(),
                                    );
                                } else {
                                    ui.label_styled(
                                        &format!("- [X] ({}) {}", done.get_date(), done.get_text()),
                                        SELECTED_PAIR,
                                        None,
                                    );
                                }
                            } else {
                                ui.label_styled(
                                    &format!("- [X]|{}| {}", done.get_date(), done.get_text()),
                                    UNSELECTED_PAIR,
                                    None,
                                );
                            }
                        } else {
                            ui.label(&format!("- [X]|{}| {}", done.get_date(), done.get_text()));
                        }
                    }
                }
                ui.end_layout();
            }
            ui.end_layout();
        }
        ui.end();

        refresh();
        let key = getch();
        if key != ERR {
            match mode {
                Mode::Normal => {
                    app.clear_message();
                    match char::from_u32(key as u32).unwrap() {
                        'k' | '\u{103}' => app.go_up(),
                        'j' | '\u{102}' => app.go_down(),
                        'g' => app.go_top(),
                        'G' => app.go_bottom(),
                        'K' => app.drag_up(),
                        'J' => app.drag_down(),
                        '\n' => app.transfer_item(),
                        'd' => app.delete_item(),
                        'i' => {
                            if let Some(cur) = app.insert_item() {
                                editing_cursor = cur;
                                mode = Mode::Edit;
                            }
                        }
                        'a' => {
                            if let Some(cur) = app.append_item() {
                                editing_cursor = cur;
                                mode = Mode::Edit;
                            }
                        }
                        'r' => {
                            if let Some(cur) = app.edit_item() {
                                editing_cursor = cur;
                                mode = Mode::Edit;
                            }
                        }
                        'u' => app.undo(),
                        '\t' => app.toggle_panel(),
                        'q' | '\u{3}' => break,
                        _ => {}
                    }
                }
                Mode::Edit => {
                    match key as u8 as char {
                        '\n' => {
                            //'\u{1b}'
                            mode = if app.finish_edit() {
                                editing_cursor = 0;
                                Mode::Normal
                            } else {
                                Mode::Edit
                            };
                        }
                        _ => app.edit_item_with(&mut editing_cursor, key),
                    }
                }
            }
        }
    }

    endwin();
    app.save(&file_path).unwrap();
}

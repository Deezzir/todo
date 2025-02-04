use std::cell::RefCell;
use std::cmp::{max, min};
use std::ops::{Add, Div, Mul, Sub};
use std::rc::Rc;

use ncurses::*;

use super::utils::truncate;

type LayoutRef = Rc<RefCell<Box<Layout>>>;

pub enum LayoutKind {
    Vert,
    Horz,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}

impl Vec2 {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    fn div_rem(self, rhs: Self) -> (Self, Self) {
        (
            Self {
                x: self.x / rhs.x,
                y: self.y / rhs.y,
            },
            Self {
                x: self.x % rhs.x,
                y: self.y % rhs.y,
            },
        )
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Mul for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
        }
    }
}

impl Div for Vec2 {
    type Output = Vec2;
    fn div(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
        }
    }
}

struct Layout {
    kind: LayoutKind,
    pos: Vec2,
    size: Vec2,
    max_size: Vec2,
    children: Vec<LayoutRef>,
}

impl Layout {
    fn new(kind: LayoutKind, pos: Vec2, max_size: Vec2) -> Self {
        Self {
            kind,
            pos,
            max_size,
            size: Vec2::default(),
            children: Vec::new(),
        }
    }

    fn available_pos(&self) -> Vec2 {
        let child_size = self.available_size().0;

        match self.kind {
            LayoutKind::Horz => {
                let x = min(self.size.x, child_size.x);
                self.pos + Vec2::new(x, 0)
            }
            LayoutKind::Vert => self.pos + self.size * Vec2::new(0, 1),
        }
    }

    fn available_size(&self) -> (Vec2, Vec2) {
        let div = self.children.len() as i32 + 1;
        match self.kind {
            LayoutKind::Horz => self.max_size.div_rem(Vec2::new(div, 1)),
            LayoutKind::Vert => self.max_size.div_rem(Vec2::new(1, div)),
        }
    }

    fn add_widget(&mut self, size: Vec2) {
        match self.kind {
            LayoutKind::Horz => {
                self.size.x += size.x;
                self.size.y = max(self.size.y, size.y);
            }
            LayoutKind::Vert => {
                self.size.x = max(self.size.x, size.x);
                self.size.y += size.y;
            }
        }
    }

    fn resize(&mut self, size: Vec2) {
        let child_size = self.available_size().0;

        self.max_size = size;
        self.size.x = min(self.size.x, child_size.x);

        for child in &self.children {
            child.borrow_mut().resize(child_size);
        }
    }

    fn add_child(&mut self, child: LayoutRef) -> Option<Vec2> {
        let child_size = child.borrow().size;
        let size = Vec2::new(child.borrow().max_size.x, child_size.y);

        self.resize(self.max_size);
        self.add_widget(size);
        self.children.push(child);

        if self.children.len() > 1 {
            Some(self.children[self.children.len() - 2].borrow().size - child_size)
        } else {
            None
        }
    }
}

pub struct UI {
    stack: Vec<LayoutRef>,
}

impl UI {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn begin(&mut self, pos: Vec2, kind: LayoutKind, max_size: Vec2) {
        assert!(self.stack.is_empty());

        let root = Box::new(Layout::new(kind, pos, max_size));
        self.stack.push(Rc::new(RefCell::new(root)));
    }

    pub fn begin_layout(&mut self, kind: LayoutKind) {
        let layout = self
            .stack
            .last()
            .expect("Can't create a layout outside of UI::begin() and UI::end()");
        let (max_size, rem) = layout.borrow().available_size();
        let child = Box::new(Layout::new(
            kind,
            layout.borrow().available_pos(),
            max_size + rem,
        ));

        self.stack.push(Rc::new(RefCell::new(child)));
    }

    pub fn br(&mut self) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render break line outside of any layout");
        let mut layout = layout.borrow_mut();
        layout.add_widget(Vec2::new(0, 1));
    }

    pub fn hl(&mut self) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render horizontal line outside of any layout");

        let text = "‾".repeat(layout.borrow().max_size.x as usize); //‾
        self.label(&text);
    }

    pub fn label(&mut self, text: &str) {
        let layout = self
            .stack
            .last()
            .expect("Tried to render label outside of any layout");
        let pos = layout.borrow().available_pos();

        let space_fill =
            " ".repeat((layout.borrow().max_size.x as usize).saturating_sub(text.len()));
        let text = truncate(text, layout.borrow().max_size.x as usize);

        mv(pos.y, pos.x);
        addstr(&format!("{text}{space_fill}"));

        layout
            .borrow_mut()
            .add_widget(Vec2::new(text.len() as i32, 1));
    }

    pub fn label_styled(&mut self, text: &str, color_pair: i16, style: Option<u32>) {
        if let Some(s) = style {
            attr_on(s);
        }
        attr_on(COLOR_PAIR(color_pair));
        self.label(text);
        attr_off(COLOR_PAIR(color_pair));
        if let Some(s) = style {
            attr_off(s);
        }
    }

    pub fn edit_label(&mut self, text: &String, cur: usize, prefix: String) {
        let layout = self
            .stack
            .last_mut()
            .expect("Tried to render edit mode outside of any layout");
        let pos = layout.borrow().available_pos();
        let space_fill =
            " ".repeat((layout.borrow().max_size.x as usize).saturating_sub(text.len()));

        // Buffer
        {
            mv(pos.y, pos.x);
            addstr(&format!("{prefix}{text}{space_fill}"));
            layout
                .borrow_mut()
                .add_widget(Vec2::new(text.len() as i32, 1));
        }
        // Cursor
        {
            mv(pos.y, pos.x + cur as i32 + prefix.len() as i32);
            attr_on(A_REVERSE());
            addstr(text.get(cur..=cur).unwrap_or(" "));
            attr_off(A_REVERSE());
        }
    }

    pub fn end_layout(&mut self) {
        let child = self
            .stack
            .pop()
            .expect("Can't end a non-existing layout. Was there UI::begin_layout()?");
        let size_diff = self
            .stack
            .last()
            .expect("Can't end a non-existing layout. Was there UI::begin_layout() or UI::begin()?")
            .borrow_mut()
            .add_child(Rc::clone(&child));

        if let Some(Vec2 { x: _, y }) = size_diff {
            if y > 0 {
                let pos = child.borrow().available_pos();
                let space_fill = " ".repeat(child.borrow().max_size.x as usize);
                for i in 0..y {
                    mv(pos.y + i, pos.x);
                    addstr(&space_fill.to_string());
                }
            }
        }
    }

    pub fn end(&mut self) {
        self.stack
            .pop()
            .expect("Can't end a non-existing UI. Was there UI::begin()?");
    }
}

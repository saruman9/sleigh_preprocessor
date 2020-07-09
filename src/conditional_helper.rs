#[derive(Debug)]
pub(crate) struct ConditionalHelper {
    in_if: bool,
    saw_else: bool,
    handled: bool,
    copy: bool,
}

impl ConditionalHelper {
    pub(crate) fn new(in_if: bool, saw_else: bool, handled: bool, copy: bool) -> Self {
        ConditionalHelper {
            in_if,
            saw_else,
            handled,
            copy,
        }
    }

    pub(crate) fn in_if(&self) -> bool {
        self.in_if
    }

    pub(crate) fn saw_else(&self) -> bool {
        self.saw_else
    }

    pub(crate) fn set_saw_else(&mut self, saw_else: bool) {
        self.saw_else = saw_else;
    }

    pub(crate) fn handled(&self) -> bool {
        self.handled
    }

    pub(crate) fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }

    pub(crate) fn copy(&self) -> bool {
        self.copy
    }

    pub(crate) fn set_copy(&mut self, copy: bool) {
        self.copy = copy;
    }
}

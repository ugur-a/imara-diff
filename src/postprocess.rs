use std::ops::Range;

use crate::intern::Token;
use crate::util::{find_hunk_end, find_hunk_start};
use crate::Diff;

impl Diff {
    pub fn postprocess_with(&mut self, before: &[Token], after: &[Token]) {
        Postprocessor {
            added: &mut self.added,
            removed: &mut self.removed,
            tokens: after,
            hunk: 0..0,
        }
        .run();
        Postprocessor {
            added: &mut self.removed,
            removed: &mut self.added,
            tokens: before,
            hunk: 0..0,
        }
        .run()
    }
}

struct Postprocessor<'a> {
    added: &'a mut [bool],
    removed: &'a [bool],
    tokens: &'a [Token],
    hunk: Range<usize>,
}

impl Postprocessor<'_> {
    fn run(mut self) {
        let mut pos_in_before = 0;
        'outer: loop {
            // find next hunk
            loop {
                if self.hunk.end >= self.added.len() {
                    debug_assert!(pos_in_before >= self.removed.len());
                    break 'outer;
                }
                self.hunk.end = find_hunk_end(self.added, self.hunk.start);
                if !self.hunk.is_empty() {
                    break;
                }
                self.hunk.start = self.hunk.end + 1;
                pos_in_before = find_hunk_end(self.removed, pos_in_before) + 1;
            }

            let mut earliest_end;
            let mut is_modification;
            loop {
                let hunk_size_unexpanded = self.hunk.len();
                while self.slide_up() {
                    pos_in_before = find_hunk_start(self.removed, pos_in_before - 1);
                }
                earliest_end = self.hunk.end;
                is_modification = self.removed[pos_in_before];
                pos_in_before = find_hunk_end(self.removed, pos_in_before);

                while self.slide_down() {
                    pos_in_before = find_hunk_end(self.removed, pos_in_before + 1);
                    is_modification |= self.removed[pos_in_before - 1];
                }

                // if this hunk was merged with another hunk we might be able to slide up/down more
                // otherwise we are done
                if hunk_size_unexpanded == self.hunk.len() {
                    break;
                }
            }

            if self.hunk.start == earliest_end {
                // there is only a single hunk position
            } else if is_modification {
                // hunk can be moved and there is a removed hunk in the same region
                // move the hunk so it align with the other hunk to produce a single
                // MODIFIED hunk instead of two seperate ADDED/REMOVED hunks
                pos_in_before = find_hunk_start(self.removed, pos_in_before - 1);
                while !self.removed[pos_in_before] {
                    self.slide_up();
                    pos_in_before -= 1;
                }
            } else {
                // this is a pure insertation that can be moved freely up and down
                // to get more intutive results apply a heuristic
            }
            self.hunk.start = self.hunk.end + 1;
            pos_in_before = find_hunk_end(self.removed, pos_in_before) + 1;
        }
    }

    fn slide_down(&mut self) -> bool {
        if self.hunk.end == self.tokens.len()
            || self.tokens[self.hunk.start] != self.tokens[self.hunk.end]
        {
            return false;
        }
        self.added[self.hunk.start] = false;
        self.added[self.hunk.end] = true;
        self.hunk.start += 1;
        self.hunk.end = find_hunk_end(self.added, self.hunk.end);
        true
    }

    fn slide_up(&mut self) -> bool {
        if self.hunk.start == 0
            || self.tokens[self.hunk.start - 1] != self.tokens[self.hunk.end - 1]
        {
            return false;
        }
        self.added[self.hunk.start - 1] = true;
        self.added[self.hunk.end - 1] = false;
        self.hunk.end -= 1;
        self.hunk.start = find_hunk_start(self.added, self.hunk.start - 1);
        true
    }
}

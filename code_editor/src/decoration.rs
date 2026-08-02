use {
    crate::text::{Edit, Length, Position},
    std::{ops::Deref, slice::Iter},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DecorationType {
    Error,
    Warning,
    Information,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Decoration {
    pub id: usize,
    pub ty: DecorationType,
    start: Position,
    end: Position,
}

impl Decoration {
    pub fn new(id: usize, start: Position, end: Position, ty: DecorationType) -> Self {
        if start > end {
            return Self {
                ty,
                id,
                start: end,
                end: start,
            };
        }
        Self { ty, id, start, end }
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn overlaps_with(self, other: Self) -> bool {
        self.end() > other.start()
    }

    pub fn length(self) -> Length {
        self.end - self.start
    }

    pub fn start(self) -> Position {
        self.start
    }

    pub fn end(self) -> Position {
        self.end
    }

    pub fn apply_edit(self, edit: &Edit) -> Self {
        Self {
            start: self.start.apply_edit(edit),
            end: self.end.apply_edit(edit),
            ..self
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DecorationSet {
    decorations: Vec<Decoration>,
}

impl DecorationSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_decorations(&self) -> &[Decoration] {
        &self.decorations
    }

    pub fn add_decoration(&mut self, decoration: Decoration) {
        if let Some(index) = self
            .decorations
            .iter()
            .position(|existing| existing.id == decoration.id)
        {
            self.decorations.remove(index);
        }
        let key = (decoration.start(), decoration.end(), decoration.id);
        let index = self
            .decorations
            .binary_search_by_key(&key, |existing| {
                (existing.start(), existing.end(), existing.id)
            })
            .unwrap_or_else(|index| index);
        self.decorations.insert(index, decoration);
    }

    pub fn clear(&mut self) {
        self.decorations.clear();
    }

    pub fn apply_edit(&mut self, edit: &Edit) {
        for decoration in &mut self.decorations {
            *decoration = decoration.apply_edit(edit);
        }
        self.decorations.sort_by_key(|decoration| {
            (decoration.start(), decoration.end(), decoration.id)
        });
    }
}

impl Default for DecorationSet {
    fn default() -> Self {
        Self {
            decorations: vec![],
        }
    }
}

impl Deref for DecorationSet {
    type Target = [Decoration];

    fn deref(&self) -> &Self::Target {
        self.decorations.as_slice()
    }
}

impl<'a> IntoIterator for &'a DecorationSet {
    type Item = &'a Decoration;
    type IntoIter = Iter<'a, Decoration>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(byte_index: usize) -> Position {
        Position {
            line_index: 0,
            byte_index,
        }
    }

    #[test]
    fn overlapping_decorations_are_retained_in_stable_order() {
        let mut decorations = DecorationSet::new();
        decorations.add_decoration(Decoration::new(
            2,
            position(1),
            position(5),
            DecorationType::Warning,
        ));
        decorations.add_decoration(Decoration::new(
            1,
            position(1),
            position(3),
            DecorationType::Error,
        ));

        assert_eq!(decorations.len(), 2);
        assert_eq!(decorations[0].id, 1);
        assert_eq!(decorations[1].id, 2);
    }

    #[test]
    fn adding_an_existing_id_replaces_and_reorders_it() {
        let mut decorations = DecorationSet::new();
        decorations.add_decoration(Decoration::new(
            7,
            position(1),
            position(2),
            DecorationType::Warning,
        ));
        decorations.add_decoration(Decoration::new(
            7,
            position(4),
            position(6),
            DecorationType::Information,
        ));

        assert_eq!(decorations.len(), 1);
        assert_eq!(decorations[0].start(), position(4));
        assert_eq!(decorations[0].ty, DecorationType::Information);
    }
}

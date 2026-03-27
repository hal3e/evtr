use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

pub(super) struct FilterState {
    indexes: Vec<usize>,
    selected_index: usize,
    query: String,
    matcher: SkimMatcherV2,
    page_scroll_size: i32,
}

impl FilterState {
    pub(super) fn new(item_count: usize, page_scroll_size: i32) -> Self {
        Self {
            indexes: (0..item_count).collect(),
            selected_index: 0,
            query: String::new(),
            matcher: SkimMatcherV2::default(),
            page_scroll_size,
        }
    }

    pub(super) fn refresh<T, F>(&mut self, items: &[T], identifier_of: F)
    where
        F: Fn(&T) -> &str,
    {
        self.indexes = filtered_indexes_by_query(items, &self.query, &self.matcher, identifier_of);
        self.selected_index = 0;
    }

    pub(super) fn has_query(&self) -> bool {
        !self.query.is_empty()
    }

    pub(super) fn move_selection_by(&mut self, delta: i32) {
        let len = self.indexes.len();
        if len == 0 || delta == 0 {
            return;
        }

        let max_index = len - 1;
        let target = self.selected_index as i32 + delta;
        self.selected_index = target.clamp(0, max_index as i32) as usize;
    }

    pub(super) fn move_up(&mut self) {
        self.move_selection_by(-1);
    }

    pub(super) fn move_down(&mut self) {
        self.move_selection_by(1);
    }

    pub(super) fn page_up(&mut self) {
        self.move_selection_by(-self.page_scroll_size);
    }

    pub(super) fn page_down(&mut self) {
        self.move_selection_by(self.page_scroll_size);
    }

    pub(super) fn select_first(&mut self) {
        self.selected_index = 0;
    }

    pub(super) fn select_last(&mut self) {
        if let Some(last_index) = self.indexes.len().checked_sub(1) {
            self.selected_index = last_index;
        }
    }

    pub(super) fn home(&mut self) {
        self.select_first();
    }

    pub(super) fn end(&mut self) {
        self.select_last();
    }

    pub(super) fn add_char(&mut self, c: char) {
        self.query.push(c);
    }

    pub(super) fn remove_char(&mut self) {
        self.query.pop();
    }

    pub(super) fn clear_search(&mut self) {
        self.query.clear();
    }

    pub(super) fn search_query(&self) -> &str {
        &self.query
    }

    pub(super) fn indexes(&self) -> &[usize] {
        &self.indexes
    }

    pub(super) fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub(super) fn selected_item_index(&self) -> Option<usize> {
        selected_item_index(&self.indexes, self.selected_index)
    }
}

fn selected_item_index(filtered_indexes: &[usize], selected_index: usize) -> Option<usize> {
    filtered_indexes.get(selected_index).copied()
}

fn filtered_indexes_by_query<T, F>(
    items: &[T],
    query: &str,
    matcher: &SkimMatcherV2,
    identifier_of: F,
) -> Vec<usize>
where
    F: Fn(&T) -> &str,
{
    if query.is_empty() {
        return (0..items.len()).collect();
    }

    let mut scored_items: Vec<(usize, i64)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            matcher
                .fuzzy_match(identifier_of(item), query)
                .map(|score| (index, score))
        })
        .collect();

    scored_items.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    scored_items.into_iter().map(|(index, _)| index).collect()
}

#[cfg(test)]
mod tests {
    use fuzzy_matcher::skim::SkimMatcherV2;

    use super::{FilterState, filtered_indexes_by_query, selected_item_index};

    #[test]
    fn filtered_indexes_by_query_returns_all_items_for_empty_query() {
        let matcher = SkimMatcherV2::default();
        let identifiers = vec!["usb mouse", "gamepad"];

        let indexes = filtered_indexes_by_query(&identifiers, "", &matcher, |item| item);

        assert_eq!(indexes, vec![0, 1]);
    }

    #[test]
    fn filtered_indexes_by_query_returns_empty_when_nothing_matches() {
        let matcher = SkimMatcherV2::default();
        let identifiers = vec!["usb mouse", "gamepad"];

        let indexes = filtered_indexes_by_query(&identifiers, "keyboard", &matcher, |item| item);

        assert!(indexes.is_empty());
    }

    #[test]
    fn selected_item_index_uses_selected_filtered_index() {
        assert_eq!(selected_item_index(&[2, 5, 7], 1), Some(5));
        assert_eq!(selected_item_index(&[2, 5, 7], 4), None);
    }

    #[test]
    fn move_selection_by_clamps_to_the_filtered_bounds() {
        let mut filter = FilterState::new(3, 10);

        filter.move_selection_by(10);
        assert_eq!(filter.selected_index(), 2);

        filter.move_selection_by(-10);
        assert_eq!(filter.selected_index(), 0);
    }

    #[test]
    fn select_last_uses_the_last_filtered_match() {
        let mut filter = FilterState::new(4, 10);
        filter.select_last();

        assert_eq!(filter.selected_item_index(), Some(3));
    }

    #[test]
    fn page_navigation_uses_the_shared_page_size() {
        let mut filter = FilterState::new(25, 10);

        filter.page_down();
        assert_eq!(filter.selected_index(), 10);

        filter.page_down();
        assert_eq!(filter.selected_index(), 20);

        filter.page_up();
        assert_eq!(filter.selected_index(), 10);
    }
}

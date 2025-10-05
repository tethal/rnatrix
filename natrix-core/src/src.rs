use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceId(usize);

#[derive(Default)]
pub struct Sources {
    sources: Vec<Source>,
}

impl Sources {
    pub fn new() -> Self {
        Self { sources: vec![] }
    }

    pub fn add_from_string(&mut self, content: &str) -> SourceId {
        let id = SourceId(self.sources.len());
        let source = Source::new(id, "<string>".to_owned(), content.to_owned());
        self.sources.push(source);
        id
    }

    pub fn get_by_id(&self, id: SourceId) -> &Source {
        &self.sources[id.0]
    }
}

pub struct Source {
    id: SourceId,
    name: String,
    content: String,
    line_starts: Vec<usize>,
}

impl Source {
    fn new(id: SourceId, name: String, content: String) -> Self {
        let mut line_starts = Vec::new();
        line_starts.push(0);
        let bytes = content.as_bytes();
        for (i, c) in bytes.iter().enumerate() {
            if c == &b'\n' {
                line_starts.push(i + 1);
            }
        }
        Source {
            id,
            name,
            content,
            line_starts,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    fn offset_to_pos(&self, offset: usize) -> (usize, usize) {
        assert!(offset <= self.content.len());
        let (line, line_start) = self.find_line_start(offset);
        (line, self.content[line_start..offset].chars().count() + 1)
    }

    fn find_line_start(&self, offset: usize) -> (usize, usize) {
        assert!(offset <= self.content.len());
        let mut a = 0usize;
        let mut b = self.line_starts.len();
        while a < b {
            let m = (a + b) / 2;
            if offset < self.line_starts[m] {
                b = m;
            } else {
                a = m + 1;
            }
        }
        (a, self.line_starts[a - 1])
    }
}

pub struct Span {
    source_id: SourceId,
    start: usize,
    end: usize,
}

impl Span {
    fn new(source: &Source, start: usize, end: usize) -> Self {
        assert!(start <= end);
        assert!(end <= source.content.len());
        Self {
            source_id: source.id,
            start,
            end,
        }
    }

    pub fn start_pos(&self, sources: &Sources) -> (usize, usize) {
        sources.get_by_id(self.source_id).offset_to_pos(self.start)
    }

    pub fn end_pos(&self, sources: &Sources) -> (usize, usize) {
        sources.get_by_id(self.source_id).offset_to_pos(self.end)
    }
}

impl Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}:{}-{}", self.source_id.0, self.start, self.end)
    }
}

pub struct Cursor<'ctx> {
    source: &'ctx Source,
    offset: usize,
}

impl<'ctx> Cursor<'ctx> {
    pub fn new(source: &'ctx Source) -> Self {
        Cursor { source, offset: 0 }
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn is_eof(&self) -> bool {
        self.offset >= self.source.content().len()
    }

    pub fn span_from(&self, start: usize) -> Span {
        assert!(start <= self.offset);
        Span::new(self.source, start, self.offset)
    }

    pub fn peek(&self) -> Option<char> {
        self.source.content[self.offset..].chars().next()
    }

    pub fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if let Some(c) = c {
            self.offset += c.len_utf8();
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coords_no_trailing_nl() {
        let mut sources = Sources::new();
        let sid = sources.add_from_string("a\nbc\ndef");
        let s = sources.get_by_id(sid);
        assert_eq!(s.offset_to_pos(0), (1, 1));
        assert_eq!(s.offset_to_pos(1), (1, 2));
        assert_eq!(s.offset_to_pos(2), (2, 1));
        assert_eq!(s.offset_to_pos(3), (2, 2));
        assert_eq!(s.offset_to_pos(4), (2, 3));
        assert_eq!(s.offset_to_pos(5), (3, 1));
        assert_eq!(s.offset_to_pos(6), (3, 2));
        assert_eq!(s.offset_to_pos(7), (3, 3));
        assert_eq!(s.offset_to_pos(8), (3, 4));
    }

    #[test]
    fn test_coords_trailing_nl() {
        let mut sources = Sources::new();
        let sid = sources.add_from_string("ab\ncd\n");
        let s = sources.get_by_id(sid);
        assert_eq!(s.offset_to_pos(0), (1, 1));
        assert_eq!(s.offset_to_pos(1), (1, 2));
        assert_eq!(s.offset_to_pos(2), (1, 3));
        assert_eq!(s.offset_to_pos(3), (2, 1));
        assert_eq!(s.offset_to_pos(4), (2, 2));
        assert_eq!(s.offset_to_pos(5), (2, 3));
        assert_eq!(s.offset_to_pos(6), (3, 1));
    }

    #[test]
    fn test_coords_crlf() {
        let mut sources = Sources::new();
        let sid = sources.add_from_string("a\r\nb\n");
        let s = sources.get_by_id(sid);
        assert_eq!(s.offset_to_pos(0), (1, 1));
        assert_eq!(s.offset_to_pos(1), (1, 2));
        assert_eq!(s.offset_to_pos(2), (1, 3));
        assert_eq!(s.offset_to_pos(3), (2, 1));
        assert_eq!(s.offset_to_pos(4), (2, 2));
        assert_eq!(s.offset_to_pos(5), (3, 1));
    }

    #[test]
    fn test_cursor() {
        let mut sources = Sources::new();
        let sid = sources.add_from_string("a");
        let s = sources.get_by_id(sid);
        let mut cursor = Cursor::new(&s);
        assert_eq!(cursor.offset(), 0);
        assert_eq!(cursor.is_eof(), false);
        assert_eq!(cursor.span_from(0).start_pos(&sources), (1, 1));
        assert_eq!(cursor.span_from(0).end_pos(&sources), (1, 1));
        assert_eq!(cursor.peek(), Some('a'));
        assert_eq!(cursor.advance(), Some('a'));

        assert_eq!(cursor.offset(), 1);
        assert_eq!(cursor.is_eof(), true);
        assert_eq!(cursor.span_from(0).start_pos(&sources), (1, 1));
        assert_eq!(cursor.span_from(0).end_pos(&sources), (1, 2));
        assert_eq!(cursor.peek(), None);
        assert_eq!(cursor.advance(), None);

        assert_eq!(cursor.offset(), 1);
        assert_eq!(cursor.is_eof(), true);
    }

    #[test]
    fn test_empty_source() {
        let mut sources = Sources::new();
        let sid = sources.add_from_string("");
        let s = sources.get_by_id(sid);
        assert_eq!(s.offset_to_pos(0), (1, 1));
        let mut cursor = Cursor::new(&s);
        assert_eq!(cursor.offset(), 0);
        assert_eq!(cursor.is_eof(), true);
        assert_eq!(cursor.peek(), None);
        assert_eq!(cursor.advance(), None);
    }

    #[test]
    fn test_unicode() {
        let mut sources = Sources::new();
        let sid = sources.add_from_string("日本語\n🦀");
        let s = sources.get_by_id(sid);
        assert_eq!(s.offset_to_pos(0), (1, 1)); // '日'
        assert_eq!(s.offset_to_pos(3), (1, 2)); // '本'
        assert_eq!(s.offset_to_pos(6), (1, 3)); // '語'
        assert_eq!(s.offset_to_pos(9), (1, 4)); // '\n'
        assert_eq!(s.offset_to_pos(10), (2, 1)); // '🦀'
        assert_eq!(s.offset_to_pos(14), (2, 2)); // eof
        let mut cursor = Cursor::new(&s);
        assert_eq!(cursor.advance(), Some('日'));
        assert_eq!(cursor.advance(), Some('本'));
        let start = cursor.offset();
        assert_eq!(cursor.advance(), Some('語'));
        assert_eq!(cursor.span_from(start).start_pos(&sources), (1, 3));
        assert_eq!(cursor.span_from(start).end_pos(&sources), (1, 4));
        assert_eq!(cursor.advance(), Some('\n'));
        assert_eq!(cursor.advance(), Some('🦀'));
        assert_eq!(cursor.advance(), None);
    }
}

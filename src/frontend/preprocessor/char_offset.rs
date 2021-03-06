use crate::meta::LocationMeta;
use crate::meta::MetaMapper;

/// stores position of characters removed from original code (ie. comments)
pub struct CharOffset {
    /// (offset start, length of this + previous offsets)
    offsets: Vec<(usize, u16)>,
}

impl CharOffset {
    fn new() -> Self {
        Self { offsets: vec![] }
    }

    /// start new offset on source_pos at original file, with length=0
    fn add_offset(&mut self, source_pos: usize) {
        let offset_to_apply = match self.offsets.len() {
            0 => 0,
            n => self.offsets[n - 1].1
        };
        let pos_without_offset = source_pos - (offset_to_apply as usize);
        self.offsets.push((pos_without_offset, 0));
    }

    /// increase length of last added offset by 1
    fn increase_offset(&mut self) {
        match self.offsets.len() {
            0 => panic!("Impossible: trying to increase non-existing offset"),
            n => {
                self.offsets[n - 1] = (self.offsets[n - 1].0, self.offsets[n - 1].1 + 1);
            }
        }
    }

    /// translate offset_pos to position in original file
    fn get_source_position(&self, offset_pos: usize) -> usize {
        let key = (offset_pos, std::u16::MAX);
        let offset_to_apply = match self.offsets.binary_search(&key) {
            Ok(idx) => {
                // apply offset of all comments before and at offset_pos
                self.offsets[idx].1
            }
            Err(idx) => {
                // apply offset of all comments before offset_pos
                if idx > 0 {
                    self.offsets[idx - 1].1
                } else {
                    0
                }
            }
        };
        offset_pos + (offset_to_apply as usize)
    }
}

impl MetaMapper<LocationMeta, LocationMeta> for CharOffset {
    fn map_meta(&self, from: &LocationMeta) -> LocationMeta {
        LocationMeta {
            offset: self.get_source_position(from.offset)
        }
    }
}

/// remove comments from source_code and remember position mapping in CharOffset
pub fn clean_comments(source_code: String) -> (String, CharOffset) {
    // containers for the results
    let mut clean_code = String::from("");
    let mut char_offset = CharOffset::new();

    // state for our simple parser
    let mut in_str = false;
    let mut erasing = false;
    let mut escaped = false;
    let mut multiline = false;
    let mut push_prev_char = false;
    let mut previous_char = '\0';

    for (idx, current_char) in source_code.chars().enumerate() {
        if !erasing {
            // handle string traversal and escaped letters
            match (in_str, escaped, current_char) {
                (false, false, '"') => {
                    // entering string
                    in_str = true;
                }
                (false, true, _) => {
                    unreachable!();
                }
                (true, false, '"') => {
                    // exiting string
                    in_str = false;
                }
                (true, false, '\\') => {
                    // next character will be escaped
                    escaped = true;
                }
                (true, true, _) => {
                    // current character is escaped
                    escaped = false;
                }
                _ => (), // ignore other cases
            };
            // handle comment begin and clean code propagation
            if in_str {
                if push_prev_char {
                    clean_code.push(previous_char);
                }
            } else {
                match (previous_char, current_char) {
                    (_, '#') => {
                        erasing = true;
                        if push_prev_char {
                            clean_code.push(previous_char);
                        }
                    }
                    ('/', '/') => {
                        erasing = true;
                        char_offset.add_offset(idx - 1);
                        char_offset.increase_offset(); // erase 1st slash
                    }
                    ('/', '*') => {
                        erasing = true;
                        multiline = true;
                        char_offset.add_offset(idx - 1);
                        char_offset.increase_offset(); // erase slash
                    }
                    _ => {
                        if push_prev_char {
                            clean_code.push(previous_char);
                        }
                    }
                };
            };
            // handle edge case: one char after finishing the multline comment
            // we can finally start pushing prev_char from the next step
            push_prev_char = true;
        } else {
            // we always erase the previous character
            char_offset.increase_offset();

            // handle comment ending
            assert!(!in_str);  // TODO: write proper tests
            if multiline && previous_char == '*' && current_char == '/' {
                // closing multiline comment, we still erase the closing characters
                char_offset.increase_offset();
                erasing = false;
                multiline = false;
                push_prev_char = false;
            } else if !multiline && current_char == '\n' {
                // closing single-line comment, we don't erase '\n' character
                erasing = false;
                push_prev_char = true;
            } else {
                // not closing the comment
                push_prev_char = false;
            }
        }
        previous_char = current_char;
    }
    if !multiline {
        // the last character is pushed regardless of being \0
        clean_code.push(previous_char);
    }
    (clean_code, char_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_comment_at_file_begin() {
        let source_code = r#"
        /* test input */

        int main() {
            int x = readInt();
            string y = readString();
            string z = readString();

            printInt(x-5);
            printString(y+z);  
            return 0 ;
        }
        "#;
        let expected_result = r#"


        int main() {
            int x = readInt();
            string y = readString();
            string z = readString();

            printInt(x-5);
            printString(y+z);  
            return 0 ;
        }
        "#;
        let input = String::from(source_code);
        let (actual_result, _) = clean_comments(input);
        assert_eq!(actual_result.trim(), String::from(expected_result).trim())
    }
}

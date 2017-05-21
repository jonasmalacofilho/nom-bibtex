//! In this module reside all the parsers need for the bibtex format.
//!
//! All the parsers are using the *nom* crates.


// Required because the compiler don't seems do recognize
// that macros are use inside of each others..
#![allow(dead_code)]

use std::str;
use nom::{alphanumeric, IResult};
use model::{Bibtex, Entry, BibliographyEntry};

/// Parse a complete bibtex file.
named!(pub bibtex<Bibtex>,
    map!(many1!(entry), |v| Bibtex::new(v))
);

/// Parse any entry in a bibtex file.
/// A good entry normally starts with a @ otherwise, it's
/// considered as a comment.
named!(pub entry<Entry>,
    ws!(
        alt!(
            do_parse!(
                peek!(char!('@')) >>
                entry: entry_with_type >>
                (entry)
            ) |
            map!(no_type_comment, |s: &str| Entry::Comment(s.into()))
        )
    )
);

/// Parse any entry which starts with a @.
fn entry_with_type<'a>(input: &'a [u8]) -> IResult<&'a [u8], Entry> {
    let entry_type = peeked_entry_type(input).to_full_result().unwrap();

    match entry_type.to_lowercase().as_ref() {
        "comment" => type_comment(input),
        "string" => variable(input),
        _ => bibliography_entry(input),
    }
}

/// Handle a comment of the format:
/// @Comment { my comment }
named!(type_comment<Entry>, do_parse!(
    entry_type >>
    comment: map_res!(inside_backet, str::from_utf8) >>
    (Entry::Comment(comment.into()))
));

/// Handle a preamble of the format:
/// @Preamble { my preamble }
named!(preamble<Entry>, do_parse!(
    entry_type >>
    preamble: map_res!(inside_backet, str::from_utf8) >>
    (Entry::Preamble(preamble.into()))
));

/// Handle a string variable from the bibtex format:
/// @String {key = "value"} or @String {key = {value}}
named!(variable<Entry>, do_parse!(
    entry_type >>
    key_val: flat_map!(inside_backet, key_value_pair) >>
    (Entry::Variable(key_val.0.into(), key_val.1.into()))
));

/// Handle a bibliography entry of the format:
/// @entry_type { citation_key,
///     tag1,
///     tag2
/// }
named!(pub bibliography_entry<Entry>, do_parse!(
    entry_t: entry_type >>
    ws!(char!('{')) >>
    citation_key: ws!(map_res!(take_until_and_consume!(","), str::from_utf8)) >>
    tags: map!(
        bib_tags,
        { |tags: Vec<(&str, &str)>| tags.iter().map(|t| convert_tuple_str_owned(t)).collect() }
    ) >>
    ws!(char!('}')) >>
    (Entry::Bibliography(BibliographyEntry::new(entry_t, citation_key, tags)))
));

/// Handle data beginning without an @ which are comments.
named!(no_type_comment<&str>,
    map!(
        map_res!(alt!(
                take_until!("@") |
                take_while!(call!(|c| c != '\0' as u8))),
            str::from_utf8),
        str::trim
    )
);

/// Parse all the tags used by one bibliography entry separated by a comma.
named!(bib_tags<Vec<(&str, &str)>>,
    separated_list!(char!(','), key_value_pair)
);

/// Parse a key-value pair which format is:
/// - key = ```{string}```
/// - key = ```"string"```
named!(key_value_pair<(&str, &str)>,
    ws!(
        separated_pair!(
            map_res!(call!(alphanumeric), str::from_utf8),
            char!('='),
            string
        )
    )
);

/// Parse a bibtex entry type which looks like:
/// @type{ ...
///
/// But don't eat the last bracket.
named!(entry_type<&str>,
    delimited!(
        char!('@'),
        map_res!(ws!(alphanumeric), str::from_utf8),
        peek!(char!('{'))
    )
);

/// Same as entry_type but with peek.
named!(peeked_entry_type<&str>,
    peek!(
        entry_type
    )
);


/// A string value in bibtex can be written in the form:
/// - ```{my string}```
/// - ```"mystring"```
named!(string<&str>,
    ws!(
        alt!(
            delimited!(
                char!('"'),
                map!(map_res!(take_until!("\""), str::from_utf8), str::trim),
                char!('"')
            ) |
            delimited!(
                char!('{'),
                map!(map_res!(take_until!("}"), str::from_utf8), str::trim),
                char!('}')
            )
        )
    )
);

/// Parse the string inside backets.
named!(inside_backet,
    ws!(
        delimited!(
            char!('{'),
            take_until!("}"),
            char!('}')
        )
    )
);

fn convert_tuple_str_owned(tuple: &(&str, &str)) -> (String, String) {
    (tuple.0.into(), tuple.1.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::IResult;

    #[test]
    fn test_entry() {
        assert_eq!(entry(b" comment"),
                   IResult::Done(&b""[..], Entry::Comment("comment".into())));

        assert_eq!(entry(b" @ StrIng { key = \"value\" }"),
                   IResult::Done(&b""[..], Entry::Variable("key".into(), "value".into())));
    }

    #[test]
    fn test_entry_with_type() {
        assert_eq!(entry_with_type(b"@Comment{test}"),
                   IResult::Done(&b""[..], Entry::Comment("test".into()))
        );

        assert_eq!(entry_with_type(b"@String{key=\"value\"}"),
                   IResult::Done(&b""[..], Entry::Variable("key".into(), "value".into()))
        );

        let example_bibtex = b"@misc{ patashnik-bibtexing,
           author = \"Oren Patashnik\",
           title = \"BIBTEXing\",
           year = \"1988\" }";

        let response = BibliographyEntry::new("misc",
                                              "patashnik-bibtexing",
                                              vec![
                ("author".into(), "Oren Patashnik".into()),
                ("title".into(), "BIBTEXing".into()),
                ("year".into(), "1988".into())
            ]);
        assert_eq!(entry_with_type(example_bibtex),
                   IResult::Done(&b""[..], Entry::Bibliography(response))
        );
    }

    #[test]
    fn test_type_comment() {
        assert_eq!(type_comment(b"@Comment{test}"),
                   IResult::Done(&b""[..], Entry::Comment("test".into())));
    }

    #[test]
    fn test_variable() {
        assert_eq!(variable(b"@string{key=\"value\"}"),
                   IResult::Done(&b""[..],
                                 Entry::Variable("key".into(), "value".into())));
    }

    #[test]
    fn test_preamble() {
        assert_eq!(preamble(b"@preamble{my preamble}"),
                   IResult::Done(&b""[..], Entry::Preamble("my preamble".into())));
    }

    #[test]
    fn test_inside_backet() {
        assert_eq!(inside_backet(b" {test}"),
                   IResult::Done(&b""[..], &b"test"[..]));
    }

    #[test]
    fn test_string() {
        assert_eq!(string(b"\"1988 \""),
                   IResult::Done(&b""[..], "1988"));

        assert_eq!(string(b"{ 1988}"),
                   IResult::Done(&b""[..], "1988"));

        assert_eq!(string(b"\"Oren Patashnik\""),
                   IResult::Done(&b""[..], "Oren Patashnik"));

        assert_eq!(string(b"{Oren Patashnik}"),
                   IResult::Done(&b""[..], "Oren Patashnik"));
    }

    #[test]
    fn test_entry_type() {
        assert_eq!(entry_type(b"@misc{"),
                   IResult::Done(&b"{"[..], "misc"));

        assert_eq!(entry_type(b"@ misc {"),
                   IResult::Done(&b"{"[..], "misc"));
    }

    #[test]
    fn test_key_value() {
        let tag_str = b"author= \"Oren Patashnik\"";

        let result = ("author", "Oren Patashnik");
        assert_eq!(key_value_pair(tag_str), IResult::Done(&b""[..], result));
    }

    #[test]
    fn test_bib_tags() {
        // A trailing char is necessary to terminate the list. See #505 in nom
        // bug tracker (https://github.com/Geal/nom/issues/505)
        let tags_str = b"author= \"Oren Patashnik\",year=\"1988\" }";

        let result = vec![
            ("author", "Oren Patashnik"),
            ("year", "1988"),
        ];
        assert_eq!(bib_tags(tags_str), IResult::Done(&b"}"[..], result));
    }

    #[test]
    fn test_no_type_comment() {
        assert_eq!(no_type_comment(b"test@"),
                   IResult::Done(&b"@"[..], "test"));
        assert_eq!(no_type_comment(b"test"),
                   IResult::Done(&b""[..], "test"));
    }

    #[test]
    fn biblio_entry() {
        let example_bibtex = b"@misc{ patashnik-bibtexing,
           author = \"Oren Patashnik\",
           title = \"BIBTEXing\",
           year = \"1988\" }";

        let response = BibliographyEntry::new("misc",
                                              "patashnik-bibtexing",
                                              vec![
                ("author".into(), "Oren Patashnik".into()),
                ("title".into(), "BIBTEXing".into()),
                ("year".into(), "1988".into())
            ]);
        assert_eq!(bibliography_entry(example_bibtex),
                   IResult::Done(&b""[..], Entry::Bibliography(response)));
    }
}

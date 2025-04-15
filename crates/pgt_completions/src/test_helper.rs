use pgt_schema_cache::SchemaCache;
use pgt_test_utils::test_database::get_new_test_db;
use sqlx::Executor;

use crate::CompletionParams;

pub static CURSOR_POS: char = '€';

pub struct InputQuery {
    sql: String,
    position: usize,
}

impl From<&str> for InputQuery {
    fn from(value: &str) -> Self {
        let position = value
            .find(CURSOR_POS)
            .expect("Insert Cursor Position into your Query.");

        InputQuery {
            sql: value.replace(CURSOR_POS, "").trim().to_string(),
            position,
        }
    }
}

impl ToString for InputQuery {
    fn to_string(&self) -> String {
        self.sql.clone()
    }
}

pub(crate) async fn get_test_deps(
    setup: &str,
    input: InputQuery,
) -> (tree_sitter::Tree, pgt_schema_cache::SchemaCache) {
    let test_db = get_new_test_db().await;

    test_db
        .execute(setup)
        .await
        .expect("Failed to execute setup query");

    let schema_cache = SchemaCache::load(&test_db)
        .await
        .expect("Failed to load Schema Cache");

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_sql::language())
        .expect("Error loading sql language");

    let tree = parser.parse(input.to_string(), None).unwrap();

    (tree, schema_cache)
}

pub(crate) fn get_text_and_position(q: InputQuery) -> (usize, String) {
    (q.position, q.sql)
}

pub(crate) fn get_test_params<'a>(
    tree: &'a tree_sitter::Tree,
    schema_cache: &'a pgt_schema_cache::SchemaCache,
    sql: InputQuery,
) -> CompletionParams<'a> {
    let (position, text) = get_text_and_position(sql);

    CompletionParams {
        position: (position as u32).into(),
        schema: schema_cache,
        tree,
        text,
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helper::CURSOR_POS;

    use super::InputQuery;

    #[test]
    fn input_query_should_extract_correct_position() {
        struct TestCase {
            query: String,
            expected_pos: usize,
            expected_sql_len: usize,
        }

        let cases = vec![
            TestCase {
                query: format!("select * from{}", CURSOR_POS),
                expected_pos: 13,
                expected_sql_len: 13,
            },
            TestCase {
                query: format!("{}select * from", CURSOR_POS),
                expected_pos: 0,
                expected_sql_len: 13,
            },
            TestCase {
                query: format!("select {} from", CURSOR_POS),
                expected_pos: 7,
                expected_sql_len: 12,
            },
        ];

        for case in cases {
            let query = InputQuery::from(case.query.as_str());
            assert_eq!(query.position, case.expected_pos);
            assert_eq!(query.sql.len(), case.expected_sql_len);
        }
    }
}

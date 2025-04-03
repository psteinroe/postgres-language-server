mod diagnostics;

pub use diagnostics::TypecheckDiagnostic;
use diagnostics::create_type_error;
use pgt_text_size::TextRange;
use sqlx::Executor;
use sqlx::PgPool;
use sqlx::postgres::PgDatabaseError;
pub use sqlx::postgres::PgSeverity;

#[derive(Debug)]
pub struct TypecheckParams<'a> {
    pub conn: &'a PgPool,
    pub sql: &'a str,
    pub ast: &'a pgt_query_ext::NodeEnum,
    pub tree: Option<&'a tree_sitter::Tree>,
}

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub code: String,
    pub severity: PgSeverity,
    pub position: Option<usize>,
    pub range: Option<TextRange>,
    pub table: Option<String>,
    pub column: Option<String>,
    pub data_type: Option<String>,
    pub constraint: Option<String>,
}

pub async fn check_sql(params: TypecheckParams<'_>) -> Option<TypecheckDiagnostic> {
    // Check if the AST is not a supported statement type
    if !matches!(
        params.ast,
        pgt_query_ext::NodeEnum::SelectStmt(_)
            | pgt_query_ext::NodeEnum::InsertStmt(_)
            | pgt_query_ext::NodeEnum::UpdateStmt(_)
            | pgt_query_ext::NodeEnum::DeleteStmt(_)
            | pgt_query_ext::NodeEnum::CommonTableExpr(_)
    ) {
        return None;
    }

    let mut conn = match params.conn.acquire().await {
        Ok(c) => c,
        Err(_) => return None,
    };

    // Postgres caches prepared statements within the current DB session (connection).
    // This can cause issues if the underlying table schema changes while statements
    // are cached. By closing the connection after use, we ensure a fresh state for
    // each typecheck operation.
    conn.close_on_drop();

    let res = conn.prepare(params.sql).await;

    match res {
        Ok(_) => None,
        Err(sqlx::Error::Database(err)) => {
            let pg_err = err.downcast_ref::<PgDatabaseError>();
            Some(create_type_error(pg_err, params.tree))
        }
        Err(_) => None,
    }
}

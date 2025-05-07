use crate::context::{CompletionContext, WrappingClause};

use super::CompletionRelevanceData;

#[derive(Debug)]
pub(crate) struct CompletionFilter<'a> {
    data: CompletionRelevanceData<'a>,
}

impl<'a> From<CompletionRelevanceData<'a>> for CompletionFilter<'a> {
    fn from(value: CompletionRelevanceData<'a>) -> Self {
        Self { data: value }
    }
}

impl CompletionFilter<'_> {
    pub fn is_relevant(&self, ctx: &CompletionContext) -> Option<()> {
        self.completable_context(ctx)?;
        self.check_clause(ctx)?;
        self.check_invocation(ctx)?;
        self.check_mentioned_schema_or_alias(ctx)?;

        Some(())
    }

    fn completable_context(&self, ctx: &CompletionContext) -> Option<()> {
        let current_node_kind = ctx.node_under_cursor.map(|n| n.kind()).unwrap_or("");

        if current_node_kind.starts_with("keyword_")
            || current_node_kind == "="
            || current_node_kind == ","
            || current_node_kind == "literal"
            || current_node_kind == "ERROR"
        {
            return None;
        }

        // No autocompletions if there are two identifiers without a separator.
        if ctx.node_under_cursor.is_some_and(|n| {
            n.prev_sibling().is_some_and(|p| {
                (p.kind() == "identifier" || p.kind() == "object_reference")
                    && n.kind() == "identifier"
            })
        }) {
            return None;
        }

        Some(())
    }

    fn check_clause(&self, ctx: &CompletionContext) -> Option<()> {
        let clause = ctx.wrapping_clause_type.as_ref();

        match self.data {
            CompletionRelevanceData::Table(_) => {
                let in_select_clause = clause.is_some_and(|c| c == &WrappingClause::Select);
                let in_where_clause = clause.is_some_and(|c| c == &WrappingClause::Where);

                if in_select_clause || in_where_clause {
                    return None;
                };
            }
            CompletionRelevanceData::Column(_) => {
                let in_from_clause = clause.is_some_and(|c| c == &WrappingClause::From);
                if in_from_clause {
                    return None;
                }

                // We can complete columns in JOIN cluases, but only if we are after the
                // ON node in the "ON u.id = posts.user_id" part.
                let in_join_clause_before_on_node = clause.is_some_and(|c| match c {
                    // we are in a JOIN, but definitely not after an ON
                    WrappingClause::Join { on_node: None } => true,

                    WrappingClause::Join { on_node: Some(on) } => ctx
                        .node_under_cursor
                        .is_some_and(|n| n.end_byte() < on.start_byte()),

                    _ => false,
                });

                if in_join_clause_before_on_node {
                    return None;
                }
            }
            _ => {}
        }

        Some(())
    }

    fn check_invocation(&self, ctx: &CompletionContext) -> Option<()> {
        if !ctx.is_invocation {
            return Some(());
        }

        match self.data {
            CompletionRelevanceData::Table(_) | CompletionRelevanceData::Column(_) => return None,
            _ => {}
        }

        Some(())
    }

    fn check_mentioned_schema_or_alias(&self, ctx: &CompletionContext) -> Option<()> {
        if ctx.schema_or_alias_name.is_none() {
            return Some(());
        }

        let schema_or_alias = ctx.schema_or_alias_name.as_ref().unwrap();

        let matches = match self.data {
            CompletionRelevanceData::Table(table) => &table.schema == schema_or_alias,
            CompletionRelevanceData::Function(f) => &f.schema == schema_or_alias,
            CompletionRelevanceData::Column(col) => ctx
                .mentioned_table_aliases
                .get(schema_or_alias)
                .is_some_and(|t| t == &col.table_name),

            CompletionRelevanceData::Schema(_) => {
                // we should never allow schema suggestions if there already was one.
                false
            }
        };

        if !matches {
            return None;
        }

        Some(())
    }
}

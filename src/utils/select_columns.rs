#[macro_export]
macro_rules! select_columns {
    (
        $ctx:expr, $query:expr,
            $(
                $($column_name:tt)|* => $column:expr
            ),*
            $(,)?
    ) => {
        $(
            if $($ctx.look_ahead().field($column_name).exists())||* {
                $query = $query.column($column);
            }
        )*
    };

    ($ctx:expr, $query:expr, $($column:tt)+) => {{
        use std::str::FromStr;

        if let Some(field) = $ctx.look_ahead().selection_fields().first() {
            for x in field.selection_set() {
                if let Ok(column) = $($column)*::from_str(x.name()) {
                    $query = $query.column(column);
                }
            }
        }
    }};
}

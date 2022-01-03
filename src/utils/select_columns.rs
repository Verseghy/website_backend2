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
    }
}

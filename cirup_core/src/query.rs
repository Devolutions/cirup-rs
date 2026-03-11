#![allow(clippy::self_named_module_files)]

use std::io;
#[cfg(test)]
use std::time::Instant;

use unicode_width::UnicodeWidthStr;

use crate::config::{QueryBackendKind, QueryConfig};
use crate::file::{
    OutputEncoding, save_resource_file, save_resource_file_with_encoding, would_save_resource_file_with_encoding,
};
use crate::query_backend::{QueryBackend, build_backend};

use crate::{Resource, Triple};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueryOutputFormat {
    Table,
    Json,
    #[default]
    Jsonl,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryRunOptions {
    pub output_format: QueryOutputFormat,
    pub count_only: bool,
    pub dry_run: bool,
    pub check: bool,
    pub summary: bool,
    pub key_filters: Vec<String>,
    pub value_filters: Vec<String>,
    pub limit: Option<usize>,
    pub operation_name: Option<String>,
    pub input_files: Vec<String>,
    pub output_file: Option<String>,
}

impl QueryRunOptions {
    #[must_use]
    pub fn with_context(mut self, operation_name: &str, input_files: &[&str], output_file: Option<&str>) -> Self {
        self.operation_name = Some(operation_name.to_owned());
        self.input_files = input_files.iter().map(|value| (*value).to_owned()).collect();
        self.output_file = output_file.map(str::to_owned);
        self
    }

    fn validate_for_output(&self, out_file: Option<&str>) -> Result<(), io::Error> {
        if self.count_only && out_file.is_some() {
            return Err(io::Error::other("--count-only cannot be combined with an output file"));
        }

        if self.count_only && self.summary {
            return Err(io::Error::other("--count-only cannot be combined with --summary"));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TextPatternSegment {
    Literal(String),
    AnyOne,
    AnyMany,
}

#[derive(Debug, Clone)]
enum TextPatternMatcher {
    Exact(String),
    Prefix(String),
    Suffix(String),
    Contains(String),
    PrefixSuffix { prefix: String, suffix: String },
    Wildcard(Vec<TextPatternSegment>),
}

impl TextPatternMatcher {
    fn matches(&self, value: &str) -> bool {
        match self {
            Self::Exact(expected) => value == expected,
            Self::Prefix(prefix) => value.starts_with(prefix),
            Self::Suffix(suffix) => value.ends_with(suffix),
            Self::Contains(fragment) => value.contains(fragment),
            Self::PrefixSuffix { prefix, suffix } => {
                value.len() >= prefix.len() + suffix.len() && value.starts_with(prefix) && value.ends_with(suffix)
            }
            Self::Wildcard(segments) => wildcard_matches(segments, value),
        }
    }
}

#[derive(Debug, Clone)]
struct CompiledTextPattern {
    matcher: TextPatternMatcher,
    glob_pattern: String,
}

impl CompiledTextPattern {
    fn matches(&self, value: &str) -> bool {
        self.matcher.matches(value)
    }
}

#[derive(Debug, Clone, Default)]
struct CompiledTextFilter {
    patterns: Vec<CompiledTextPattern>,
}

impl CompiledTextFilter {
    fn matches(&self, value: &str) -> bool {
        self.patterns.iter().any(|pattern| pattern.matches(value))
    }

    fn sql_condition(&self, value_expr: &str) -> String {
        let mut clauses = self.patterns.iter();
        let Some(first_pattern) = clauses.next() else {
            return String::new();
        };

        let mut condition = String::with_capacity((value_expr.len() + 24) * self.patterns.len() + 4);
        let needs_wrap = self.patterns.len() > 1;

        if needs_wrap {
            condition.push('(');
        }

        append_glob_clause(&mut condition, value_expr, &first_pattern.glob_pattern);

        for pattern in clauses {
            condition.push_str(" OR ");
            append_glob_clause(&mut condition, value_expr, &pattern.glob_pattern);
        }

        if needs_wrap {
            condition.push(')');
        }

        condition
    }

    fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

fn make_error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

fn append_sql_quote_literal(output: &mut String, value: &str) {
    output.push('\'');
    for ch in value.chars() {
        if ch == '\'' {
            output.push('\'');
        }
        output.push(ch);
    }
    output.push('\'');
}

fn append_glob_clause(output: &mut String, value_expr: &str, glob_pattern: &str) {
    output.push_str(value_expr);
    output.push_str(" GLOB ");
    append_sql_quote_literal(output, glob_pattern);
}

fn append_escaped_glob_char(output: &mut String, ch: char) {
    match ch {
        '*' => output.push_str("[*]"),
        '?' => output.push_str("[?]"),
        '[' => output.push_str("[[]"),
        ']' => output.push_str("[]]"),
        _ => output.push(ch),
    }
}

fn push_glob_wildcard(output: &mut String, previous_star: &mut bool) {
    if !*previous_star {
        output.push('*');
        *previous_star = true;
    }
}

fn is_escaped_char(chars: &[char], index: usize) -> bool {
    let mut backslash_count = 0;
    let mut cursor = index;

    while cursor > 0 {
        cursor -= 1;
        if chars[cursor] == '\\' {
            backslash_count += 1;
        } else {
            break;
        }
    }

    backslash_count % 2 == 1
}

fn push_match_segment(output: &mut Vec<TextPatternSegment>, segment: TextPatternSegment) {
    if matches!(segment, TextPatternSegment::AnyMany) && matches!(output.last(), Some(TextPatternSegment::AnyMany)) {
        return;
    }

    output.push(segment);
}

fn advance_char_boundary(value: &str, index: usize) -> Option<usize> {
    value[index..].chars().next().map(|ch| index + ch.len_utf8())
}

fn wildcard_matches(segments: &[TextPatternSegment], value: &str) -> bool {
    let mut segment_index = 0usize;
    let mut value_index = 0usize;
    let mut star_segment_index = None;
    let mut star_value_index = 0usize;

    while value_index < value.len() {
        match segments.get(segment_index) {
            Some(TextPatternSegment::Literal(literal)) if value[value_index..].starts_with(literal) => {
                value_index += literal.len();
                segment_index += 1;
            }
            Some(TextPatternSegment::AnyOne) => {
                let Some(next_index) = advance_char_boundary(value, value_index) else {
                    return false;
                };
                value_index = next_index;
                segment_index += 1;
            }
            Some(TextPatternSegment::AnyMany) => {
                star_segment_index = Some(segment_index);
                star_value_index = value_index;
                segment_index += 1;
            }
            _ => {
                let Some(star_index) = star_segment_index else {
                    return false;
                };
                let Some(next_index) = advance_char_boundary(value, star_value_index) else {
                    return false;
                };
                star_value_index = next_index;
                value_index = next_index;
                segment_index = star_index + 1;
            }
        }
    }

    while matches!(segments.get(segment_index), Some(TextPatternSegment::AnyMany)) {
        segment_index += 1;
    }

    segment_index == segments.len()
}

fn matcher_from_segments(segments: Vec<TextPatternSegment>) -> TextPatternMatcher {
    match segments.as_slice() {
        [] => TextPatternMatcher::Exact(String::new()),
        [TextPatternSegment::AnyMany] => TextPatternMatcher::Contains(String::new()),
        [TextPatternSegment::Literal(literal)] => TextPatternMatcher::Exact(literal.clone()),
        [TextPatternSegment::Literal(prefix), TextPatternSegment::AnyMany] => {
            TextPatternMatcher::Prefix(prefix.clone())
        }
        [TextPatternSegment::AnyMany, TextPatternSegment::Literal(suffix)] => {
            TextPatternMatcher::Suffix(suffix.clone())
        }
        [
            TextPatternSegment::AnyMany,
            TextPatternSegment::Literal(fragment),
            TextPatternSegment::AnyMany,
        ] => TextPatternMatcher::Contains(fragment.clone()),
        [
            TextPatternSegment::Literal(prefix),
            TextPatternSegment::AnyMany,
            TextPatternSegment::Literal(suffix),
        ] => TextPatternMatcher::PrefixSuffix {
            prefix: prefix.clone(),
            suffix: suffix.clone(),
        },
        _ => TextPatternMatcher::Wildcard(segments),
    }
}

fn compile_text_pattern(flag_name: &str, pattern: &str) -> Result<CompiledTextPattern, io::Error> {
    if pattern.is_empty() {
        return Err(make_error(format!("{flag_name} cannot be empty")));
    }

    let chars = pattern.chars().collect::<Vec<_>>();
    let anchored_start = chars.first() == Some(&'^');
    let anchored_end = chars.last() == Some(&'$') && !is_escaped_char(&chars, chars.len() - 1);

    let start_index = usize::from(anchored_start);
    let end_index = if anchored_end { chars.len() - 1 } else { chars.len() };

    if start_index > end_index {
        return Err(make_error(format!(
            "invalid {flag_name} '{}': missing pattern body",
            pattern
        )));
    }

    let mut segments = Vec::new();
    let mut literal_buffer = String::new();
    let mut index = start_index;

    while index < end_index {
        let ch = chars[index];

        if ch == '\\' {
            index += 1;
            if index >= end_index {
                return Err(make_error(format!(
                    "invalid {flag_name} '{}': trailing escape sequence",
                    pattern,
                )));
            }

            literal_buffer.push(chars[index]);
            index += 1;
            continue;
        }

        if ch == '.' {
            if !literal_buffer.is_empty() {
                segments.push(TextPatternSegment::Literal(std::mem::take(&mut literal_buffer)));
            }

            if index + 1 < end_index && chars[index + 1] == '*' {
                segments.push(TextPatternSegment::AnyMany);
                index += 2;
            } else {
                segments.push(TextPatternSegment::AnyOne);
                index += 1;
            }
            continue;
        }

        if matches!(ch, '*' | '+' | '?' | '|' | '(' | ')' | '[' | ']' | '{' | '}') {
            return Err(make_error(format!(
                "invalid {flag_name} '{}': unsupported syntax '{}'; supported syntax is literals, ^, $, . and .*",
                pattern, ch,
            )));
        }

        if ch == '^' {
            return Err(make_error(format!(
                "invalid {flag_name} '{}': '^' is only supported at the start",
                pattern
            )));
        }

        if ch == '$' {
            return Err(make_error(format!(
                "invalid {flag_name} '{}': '$' is only supported at the end",
                pattern
            )));
        }

        literal_buffer.push(ch);
        index += 1;
    }

    if !literal_buffer.is_empty() {
        segments.push(TextPatternSegment::Literal(literal_buffer));
    }

    let mut glob_pattern = String::with_capacity(pattern.len() + 2);
    let mut previous_glob_star = false;
    let mut match_segments = Vec::with_capacity(segments.len() + 2);

    if !anchored_start {
        push_glob_wildcard(&mut glob_pattern, &mut previous_glob_star);
        push_match_segment(&mut match_segments, TextPatternSegment::AnyMany);
    }

    for segment in &segments {
        match segment {
            TextPatternSegment::Literal(value) => {
                for ch in value.chars() {
                    append_escaped_glob_char(&mut glob_pattern, ch);
                    previous_glob_star = false;
                }
                push_match_segment(&mut match_segments, TextPatternSegment::Literal(value.clone()));
            }
            TextPatternSegment::AnyOne => {
                glob_pattern.push('?');
                previous_glob_star = false;
                push_match_segment(&mut match_segments, TextPatternSegment::AnyOne);
            }
            TextPatternSegment::AnyMany => {
                push_glob_wildcard(&mut glob_pattern, &mut previous_glob_star);
                push_match_segment(&mut match_segments, TextPatternSegment::AnyMany);
            }
        }
    }

    if !anchored_end {
        push_glob_wildcard(&mut glob_pattern, &mut previous_glob_star);
        push_match_segment(&mut match_segments, TextPatternSegment::AnyMany);
    }

    Ok(CompiledTextPattern {
        matcher: matcher_from_segments(match_segments),
        glob_pattern,
    })
}

fn compile_text_filter(flag_name: &str, patterns: &[String]) -> Result<Option<CompiledTextFilter>, io::Error> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let patterns = patterns
        .iter()
        .map(|pattern| compile_text_pattern(flag_name, pattern))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(CompiledTextFilter { patterns }))
}

#[derive(Debug, Clone, Default)]
struct CompiledQueryFilters {
    key_filter: Option<CompiledTextFilter>,
    value_filter: Option<CompiledTextFilter>,
}

impl CompiledQueryFilters {
    fn is_empty(&self) -> bool {
        self.key_filter.is_none() && self.value_filter.is_none()
    }
}

fn compile_query_filters(options: &QueryRunOptions) -> Result<CompiledQueryFilters, io::Error> {
    Ok(CompiledQueryFilters {
        key_filter: compile_text_filter("--key-filter", &options.key_filters)?,
        value_filter: compile_text_filter("--value-filter", &options.value_filters)?,
    })
}

fn canonical_sql(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut saw_token = false;

    for token in input.split_whitespace() {
        if saw_token {
            output.push(' ');
        }

        for ch in token.chars() {
            output.push(ch.to_ascii_lowercase());
        }

        saw_token = true;
    }

    output
}

fn wrap_resource_query_with_filters(query: &str, filters: &CompiledQueryFilters) -> String {
    if filters.is_empty() {
        return query.to_owned();
    }

    let mut conditions = Vec::new();

    if let Some(key_filter) = filters.key_filter.as_ref()
        && !key_filter.is_empty()
    {
        conditions.push(key_filter.sql_condition("filtered.key"));
    }

    if let Some(value_filter) = filters.value_filter.as_ref()
        && !value_filter.is_empty()
    {
        conditions.push(value_filter.sql_condition("filtered.val"));
    }

    if conditions.is_empty() {
        return query.to_owned();
    }

    let mut wrapped = format!(
        "WITH filtered(key, val) AS ({query}) SELECT key, val FROM filtered WHERE {}",
        conditions.join(" AND ")
    );

    if canonical_sql(query) == canonical_sql(SORT_QUERY) {
        wrapped.push_str(" ORDER BY key");
    }

    wrapped
}

fn wrap_triple_query_with_filters(query: &str, filters: &CompiledQueryFilters) -> String {
    if filters.is_empty() {
        return query.to_owned();
    }

    let mut conditions = Vec::new();

    if let Some(key_filter) = filters.key_filter.as_ref()
        && !key_filter.is_empty()
    {
        conditions.push(key_filter.sql_condition("filtered.key"));
    }

    if let Some(value_filter) = filters.value_filter.as_ref()
        && !value_filter.is_empty()
    {
        conditions.push(value_filter.sql_condition("filtered.val"));
    }

    if conditions.is_empty() {
        return query.to_owned();
    }

    format!(
        "WITH filtered(key, val, base) AS ({query}) SELECT key, val, base FROM filtered WHERE {}",
        conditions.join(" AND ")
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueryExecutionCounts {
    matched_count: usize,
    filtered_count: usize,
    output_count: usize,
    truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct QueryExecutionReport {
    pub operation: Option<String>,
    pub result_kind: String,
    pub input_files: Vec<String>,
    pub output_file: Option<String>,
    pub matched_count: usize,
    pub filtered_count: usize,
    pub output_count: usize,
    pub truncated: bool,
    pub dry_run: bool,
    pub check: bool,
    pub would_write: bool,
    pub wrote_output: bool,
    pub change_detected: bool,
}

impl QueryExecutionReport {
    fn from_options(
        options: &QueryRunOptions,
        result_kind: &str,
        counts: QueryExecutionCounts,
        would_write: bool,
        wrote_output: bool,
    ) -> Self {
        let change_detected = if options.output_file.is_some() {
            would_write
        } else {
            counts.output_count > 0
        };

        Self {
            operation: options.operation_name.clone(),
            result_kind: result_kind.to_owned(),
            input_files: options.input_files.clone(),
            output_file: options.output_file.clone(),
            matched_count: counts.matched_count,
            filtered_count: counts.filtered_count,
            output_count: counts.output_count,
            truncated: counts.truncated,
            dry_run: options.dry_run,
            check: options.check,
            would_write,
            wrote_output,
            change_detected,
        }
    }

    pub fn indicates_change(&self) -> bool {
        self.change_detected
    }
}

fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }

    text
}

fn ascii_table_border<const N: usize>(output: &mut String, widths: &[usize; N]) {
    output.push('+');
    for width in widths {
        for _ in 0..(*width + 2) {
            output.push('-');
        }
        output.push('+');
    }
    output.push('\n');
}

fn ascii_table_row<const N: usize>(output: &mut String, widths: &[usize; N], cells: [&str; N]) {
    output.push('|');
    for (width, cell) in widths.iter().zip(cells) {
        output.push(' ');
        output.push_str(cell);
        let padding = width.saturating_sub(UnicodeWidthStr::width(cell)) + 1;
        for _ in 0..padding {
            output.push(' ');
        }
        output.push('|');
    }
    output.push('\n');
}

fn estimate_ascii_table_capacity(column_widths: &[usize], row_count: usize) -> usize {
    let line_len = column_widths.iter().sum::<usize>() + column_widths.len() * 3 + 2;
    line_len * (row_count * 2 + 1)
}

fn resources_to_table(resources: &[Resource]) -> String {
    let mut widths = [UnicodeWidthStr::width("name"), UnicodeWidthStr::width("value")];

    for resource in resources {
        widths[0] = widths[0].max(UnicodeWidthStr::width(resource.name.as_str()));
        widths[1] = widths[1].max(UnicodeWidthStr::width(resource.value.as_str()));
    }

    let mut output = String::with_capacity(estimate_ascii_table_capacity(&widths, resources.len() + 1));

    ascii_table_border(&mut output, &widths);
    ascii_table_row(&mut output, &widths, ["name", "value"]);
    ascii_table_border(&mut output, &widths);

    for resource in resources {
        ascii_table_row(&mut output, &widths, [resource.name.as_str(), resource.value.as_str()]);
        ascii_table_border(&mut output, &widths);
    }

    output
}

fn triples_to_table(triples: &[Triple]) -> String {
    let mut widths = [
        UnicodeWidthStr::width("name"),
        UnicodeWidthStr::width("value"),
        UnicodeWidthStr::width("base"),
    ];

    for triple in triples {
        widths[0] = widths[0].max(UnicodeWidthStr::width(triple.name.as_str()));
        widths[1] = widths[1].max(UnicodeWidthStr::width(triple.value.as_str()));
        widths[2] = widths[2].max(UnicodeWidthStr::width(triple.base.as_str()));
    }

    let mut output = String::with_capacity(estimate_ascii_table_capacity(&widths, triples.len() + 1));

    ascii_table_border(&mut output, &widths);
    ascii_table_row(&mut output, &widths, ["name", "value", "base"]);
    ascii_table_border(&mut output, &widths);

    for triple in triples {
        ascii_table_row(
            &mut output,
            &widths,
            [triple.name.as_str(), triple.value.as_str(), triple.base.as_str()],
        );
        ascii_table_border(&mut output, &widths);
    }

    output
}

fn render_jsonl<T: serde::Serialize>(values: &[T]) -> String {
    let mut output = Vec::with_capacity(values.len().saturating_mul(32));

    for value in values {
        serde_json::to_writer(&mut output, value).expect("failed to serialize JSONL row");
        output.push(b'\n');
    }

    // SAFETY: serde_json emits valid UTF-8 and this function only appends ASCII newlines.
    unsafe { String::from_utf8_unchecked(output) }
}

fn render_resources(resources: &[Resource], output_format: QueryOutputFormat) -> String {
    match output_format {
        QueryOutputFormat::Table => resources_to_table(resources),
        QueryOutputFormat::Json => ensure_trailing_newline(
            serde_json::to_string(resources).expect("failed to serialize resource list to JSON"),
        ),
        QueryOutputFormat::Jsonl => render_jsonl(resources),
    }
}

fn render_triples(triples: &[Triple], output_format: QueryOutputFormat) -> String {
    match output_format {
        QueryOutputFormat::Table => triples_to_table(triples),
        QueryOutputFormat::Json => {
            ensure_trailing_newline(serde_json::to_string(triples).expect("failed to serialize triple list to JSON"))
        }
        QueryOutputFormat::Jsonl => render_jsonl(triples),
    }
}

fn render_count(count: usize) -> String {
    format!("{count}\n")
}

fn bool_str(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn report_to_table(report: &QueryExecutionReport) -> String {
    let input_files = report.input_files.join(",");
    let matched_count = report.matched_count.to_string();
    let filtered_count = report.filtered_count.to_string();
    let output_count = report.output_count.to_string();
    let rows = [
        ("operation", report.operation.as_deref().unwrap_or_default()),
        ("result_kind", report.result_kind.as_str()),
        ("input_files", input_files.as_str()),
        ("output_file", report.output_file.as_deref().unwrap_or_default()),
        ("matched_count", matched_count.as_str()),
        ("filtered_count", filtered_count.as_str()),
        ("output_count", output_count.as_str()),
        ("truncated", bool_str(report.truncated)),
        ("dry_run", bool_str(report.dry_run)),
        ("check", bool_str(report.check)),
        ("would_write", bool_str(report.would_write)),
        ("wrote_output", bool_str(report.wrote_output)),
        ("change_detected", bool_str(report.change_detected)),
    ];

    let mut widths = [UnicodeWidthStr::width("field"), UnicodeWidthStr::width("value")];
    for (field, value) in rows {
        widths[0] = widths[0].max(UnicodeWidthStr::width(field));
        widths[1] = widths[1].max(UnicodeWidthStr::width(value));
    }

    let mut output = String::with_capacity(estimate_ascii_table_capacity(&widths, rows.len() + 1));
    ascii_table_border(&mut output, &widths);
    ascii_table_row(&mut output, &widths, ["field", "value"]);
    ascii_table_border(&mut output, &widths);

    for (field, value) in rows {
        ascii_table_row(&mut output, &widths, [field, value]);
        ascii_table_border(&mut output, &widths);
    }

    output
}

fn render_report(report: &QueryExecutionReport, output_format: QueryOutputFormat) -> String {
    match output_format {
        QueryOutputFormat::Table => report_to_table(report),
        QueryOutputFormat::Json => ensure_trailing_newline(
            serde_json::to_string(report).expect("failed to serialize execution report to JSON"),
        ),
        QueryOutputFormat::Jsonl => render_jsonl(std::slice::from_ref(report)),
    }
}

fn filter_resources(
    mut resources: Vec<Resource>,
    filters: &CompiledQueryFilters,
    limit: Option<usize>,
) -> (QueryExecutionCounts, Vec<Resource>) {
    let matched_count = resources.len();
    match (filters.key_filter.as_ref(), filters.value_filter.as_ref()) {
        (Some(key_filter), Some(value_filter)) => {
            resources.retain(|resource| key_filter.matches(&resource.name) && value_filter.matches(&resource.value));
        }
        (Some(key_filter), None) => {
            resources.retain(|resource| key_filter.matches(&resource.name));
        }
        (None, Some(value_filter)) => {
            resources.retain(|resource| value_filter.matches(&resource.value));
        }
        (None, None) => {}
    }

    let filtered_count = resources.len();
    let mut truncated = false;

    if let Some(limit) = limit {
        truncated = filtered_count > limit;
        resources.truncate(limit);
    }

    (
        QueryExecutionCounts {
            matched_count,
            filtered_count,
            output_count: resources.len(),
            truncated,
        },
        resources,
    )
}

fn filter_triples(
    mut triples: Vec<Triple>,
    filters: &CompiledQueryFilters,
    limit: Option<usize>,
) -> (QueryExecutionCounts, Vec<Triple>) {
    let matched_count = triples.len();
    match (filters.key_filter.as_ref(), filters.value_filter.as_ref()) {
        (Some(key_filter), Some(value_filter)) => {
            triples.retain(|triple| key_filter.matches(&triple.name) && value_filter.matches(&triple.value));
        }
        (Some(key_filter), None) => {
            triples.retain(|triple| key_filter.matches(&triple.name));
        }
        (None, Some(value_filter)) => {
            triples.retain(|triple| value_filter.matches(&triple.value));
        }
        (None, None) => {}
    }

    let filtered_count = triples.len();
    let mut truncated = false;

    if let Some(limit) = limit {
        truncated = filtered_count > limit;
        triples.truncate(limit);
    }

    (
        QueryExecutionCounts {
            matched_count,
            filtered_count,
            output_count: triples.len(),
            truncated,
        },
        triples,
    )
}

#[allow(clippy::print_stdout)]
pub fn print_resources_pretty(resources: &[Resource]) {
    print!("{}", resources_to_table(resources));
}

#[allow(clippy::print_stdout)]
pub fn print_triples_pretty(triples: &[Triple]) {
    print!("{}", triples_to_table(triples));
}

fn default_query_backend() -> QueryBackendKind {
    std::env::var("CIRUP_QUERY_BACKEND")
        .ok()
        .and_then(|value| QueryBackendKind::parse(&value))
        .unwrap_or_default()
}

fn default_query_config() -> QueryConfig {
    let mut query_config = QueryConfig {
        backend: default_query_backend(),
        ..QueryConfig::default()
    };

    query_config.turso.url = std::env::var("CIRUP_TURSO_URL")
        .ok()
        .or_else(|| std::env::var("LIBSQL_URL").ok())
        .or_else(|| std::env::var("LIBSQL_HRANA_URL").ok());
    query_config.turso.auth_token = std::env::var("CIRUP_TURSO_AUTH_TOKEN")
        .ok()
        .or_else(|| std::env::var("LIBSQL_AUTH_TOKEN").ok())
        .or_else(|| std::env::var("TURSO_AUTH_TOKEN").ok());

    query_config
}

pub fn query_file(input: &str, table: &str, query: &str) {
    let mut engine = CirupEngine::new();
    engine.register_table_from_file(table, input);
    let resources = engine.query_resource(query);
    print_resources_pretty(&resources);
}

pub struct CirupEngine {
    backend: Box<dyn QueryBackend>,
}

impl CirupEngine {
    pub fn new() -> Self {
        Self::with_query_config(&default_query_config())
    }

    pub fn with_backend(kind: QueryBackendKind) -> Self {
        let mut query_config = default_query_config();
        query_config.backend = kind;
        Self::with_query_config(&query_config)
    }

    pub fn with_query_config(query_config: &QueryConfig) -> Self {
        Self {
            backend: build_backend(query_config),
        }
    }

    #[cfg(test)]
    fn register_table_from_str(&mut self, table: &str, filename: &str, data: &str) {
        self.backend.register_table_from_str(table, filename, data);
    }

    pub fn register_table_from_file(&mut self, table: &str, filename: &str) {
        self.backend.register_table_from_file(table, filename);
    }

    pub fn query_resource(&self, query: &str) -> Vec<Resource> {
        self.backend.query_resource(query)
    }

    pub fn query_triple(&self, query: &str) -> Vec<Triple> {
        self.backend.query_triple(query)
    }
}

impl Default for CirupEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CirupQuery {
    engine: CirupEngine,
    query: String,
}

const PRINT_QUERY: &str = "select * from a";
const DIFF_QUERY: &str = "select a.key, a.val, b.val from a left outer join b on a.key = b.key where (b.val is null)";
const DIFF_WITH_BASE_QUERY: &str = "select b.key, b.val, c.val from b left outer join a on b.key = a.key inner join c on b.key = c.key where (a.val is null)";
const CHANGE_QUERY: &str =
    "select a.key, a.val, b.val from a left outer join b on a.key = b.key where (b.val is null) or (a.val <> b.val)";
const MERGE_QUERY: &str = "select a.key, case when b.val is not null then b.val else a.val end from a left outer join b on a.key = b.key union select b.key, b.val from b left outer join a on a.key = b.key where (a.key is null)";
const INTERSECT_QUERY: &str = "select * from a intersect select * from b";
const SUBTRACT_QUERY: &str = "select * from a where a.key not in (select b.key from b)";
const CONVERT_QUERY: &str = PRINT_QUERY;
const SORT_QUERY: &str = "select * from a order by a.key";

pub fn query_print(file: &str) -> CirupQuery {
    query_print_with_backend(file, default_query_backend())
}

pub fn query_print_with_backend(file: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(PRINT_QUERY, file, None, None, backend)
}

pub fn query_convert(file: &str) -> CirupQuery {
    query_convert_with_backend(file, default_query_backend())
}

pub fn query_convert_with_backend(file: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(CONVERT_QUERY, file, None, None, backend)
}

pub fn query_sort(file: &str) -> CirupQuery {
    query_sort_with_backend(file, default_query_backend())
}

pub fn query_sort_with_backend(file: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(SORT_QUERY, file, None, None, backend)
}

pub fn query_diff(file_one: &str, file_two: &str) -> CirupQuery {
    query_diff_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_diff_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(DIFF_QUERY, file_one, Some(file_two), None, backend)
}

pub fn query_diff_with_config(file_one: &str, file_two: &str, query_config: &QueryConfig) -> CirupQuery {
    CirupQuery::new_with_query_config(DIFF_QUERY, file_one, Some(file_two), None, query_config)
}

pub fn query_diff_with_base(old: &str, new: &str, base: &str) -> CirupQuery {
    query_diff_with_base_with_backend(old, new, base, default_query_backend())
}

pub fn query_diff_with_base_with_backend(old: &str, new: &str, base: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(DIFF_WITH_BASE_QUERY, old, Some(new), Some(base), backend)
}

pub fn query_change(file_one: &str, file_two: &str) -> CirupQuery {
    query_change_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_change_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(CHANGE_QUERY, file_one, Some(file_two), None, backend)
}

pub fn query_change_with_config(file_one: &str, file_two: &str, query_config: &QueryConfig) -> CirupQuery {
    CirupQuery::new_with_query_config(CHANGE_QUERY, file_one, Some(file_two), None, query_config)
}

pub fn query_merge(file_one: &str, file_two: &str) -> CirupQuery {
    query_merge_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_merge_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(MERGE_QUERY, file_one, Some(file_two), None, backend)
}

pub fn query_merge_with_config(file_one: &str, file_two: &str, query_config: &QueryConfig) -> CirupQuery {
    CirupQuery::new_with_query_config(MERGE_QUERY, file_one, Some(file_two), None, query_config)
}

pub fn query_intersect(file_one: &str, file_two: &str) -> CirupQuery {
    query_intersect_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_intersect_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(INTERSECT_QUERY, file_one, Some(file_two), None, backend)
}

pub fn query_subtract(file_one: &str, file_two: &str) -> CirupQuery {
    query_subtract_with_backend(file_one, file_two, default_query_backend())
}

pub fn query_subtract_with_backend(file_one: &str, file_two: &str, backend: QueryBackendKind) -> CirupQuery {
    CirupQuery::new_with_backend(SUBTRACT_QUERY, file_one, Some(file_two), None, backend)
}

impl CirupQuery {
    pub fn new(query: &str, file_one: &str, file_two: Option<&str>, file_three: Option<&str>) -> Self {
        Self::new_with_query_config(query, file_one, file_two, file_three, &default_query_config())
    }

    pub fn new_with_backend(
        query: &str,
        file_one: &str,
        file_two: Option<&str>,
        file_three: Option<&str>,
        backend: QueryBackendKind,
    ) -> Self {
        let mut query_config = default_query_config();
        query_config.backend = backend;
        Self::new_with_query_config(query, file_one, file_two, file_three, &query_config)
    }

    pub fn new_with_query_config(
        query: &str,
        file_one: &str,
        file_two: Option<&str>,
        file_three: Option<&str>,
        query_config: &QueryConfig,
    ) -> Self {
        let mut engine = CirupEngine::with_query_config(query_config);
        engine.register_table_from_file("A", file_one);

        if let Some(file_two) = file_two {
            engine.register_table_from_file("B", file_two);
        }

        if let Some(file_three) = file_three {
            engine.register_table_from_file("C", file_three);
        }

        CirupQuery {
            engine,
            query: query.to_owned(),
        }
    }

    pub fn run(&self) -> Vec<Resource> {
        self.engine.query_resource(&self.query)
    }

    pub fn run_triple(&self) -> Vec<Triple> {
        self.engine.query_triple(&self.query)
    }

    pub fn run_with_options(&self, options: &QueryRunOptions) -> Vec<Resource> {
        let filters = compile_query_filters(options).expect("invalid text filter");
        let query = wrap_resource_query_with_filters(&self.query, &filters);
        let (_, resources) = filter_resources(self.engine.query_resource(&query), &filters, options.limit);
        resources
    }

    pub fn run_triple_with_options(&self, options: &QueryRunOptions) -> Vec<Triple> {
        let filters = compile_query_filters(options).expect("invalid text filter");
        let query = wrap_triple_query_with_filters(&self.query, &filters);
        let (_, triples) = filter_triples(self.engine.query_triple(&query), &filters, options.limit);
        triples
    }

    pub fn run_interactive(&self, out_file: Option<&str>, touch: bool) {
        let resources = self.run();

        if let Some(out_file) = out_file {
            save_resource_file(out_file, &resources, touch);
        } else {
            print_resources_pretty(&resources);
        }
    }

    pub fn run_interactive_with_encoding(&self, out_file: Option<&str>, touch: bool, output_encoding: OutputEncoding) {
        let resources = self.run();

        if let Some(out_file) = out_file {
            save_resource_file_with_encoding(out_file, &resources, touch, output_encoding);
        } else {
            print_resources_pretty(&resources);
        }
    }

    #[allow(clippy::print_stdout)]
    pub fn run_interactive_with_options(
        &self,
        out_file: Option<&str>,
        touch: bool,
        output_encoding: OutputEncoding,
        options: &QueryRunOptions,
    ) -> Result<QueryExecutionReport, io::Error> {
        options.validate_for_output(out_file)?;
        let filters = compile_query_filters(options)?;
        let query = wrap_resource_query_with_filters(&self.query, &filters);

        let (counts, resources) = filter_resources(self.engine.query_resource(&query), &filters, options.limit);
        let would_write = out_file
            .map(|path| would_save_resource_file_with_encoding(path, &resources, touch, output_encoding))
            .unwrap_or(false);
        let mut wrote_output = false;
        let report = QueryExecutionReport::from_options(options, "resource", counts, would_write, false);

        if options.count_only {
            print!("{}", render_count(counts.output_count));
            return Ok(report);
        }

        if options.check {
            if options.summary {
                print!("{}", render_report(&report, options.output_format));
            }
            return Ok(report);
        }

        if let Some(out_file) = out_file {
            if options.dry_run {
                if !options.summary {
                    print!("{}", render_resources(&resources, options.output_format));
                }
            } else {
                save_resource_file_with_encoding(out_file, &resources, touch, output_encoding);
                wrote_output = would_write;
            }
        } else if !options.summary {
            print!("{}", render_resources(&resources, options.output_format));
        }

        let report = QueryExecutionReport::from_options(options, "resource", counts, would_write, wrote_output);

        if options.summary {
            print!("{}", render_report(&report, options.output_format));
        }

        Ok(report)
    }

    pub fn run_triple_interactive(&self) {
        let triples = self.run_triple();
        print_triples_pretty(&triples);
    }

    #[allow(clippy::print_stdout)]
    pub fn run_triple_interactive_with_options(
        &self,
        options: &QueryRunOptions,
    ) -> Result<QueryExecutionReport, io::Error> {
        options.validate_for_output(None)?;
        let filters = compile_query_filters(options)?;
        let query = wrap_triple_query_with_filters(&self.query, &filters);

        let (counts, triples) = filter_triples(self.engine.query_triple(&query), &filters, options.limit);
        let report = QueryExecutionReport::from_options(options, "triple", counts, false, false);

        if options.count_only {
            print!("{}", render_count(counts.output_count));
            return Ok(report);
        }

        if options.check {
            if options.summary {
                print!("{}", render_report(&report, options.output_format));
            }
            return Ok(report);
        }

        if !options.summary {
            print!("{}", render_triples(&triples, options.output_format));
        }

        if options.summary {
            print!("{}", render_report(&report, options.output_format));
        }

        Ok(report)
    }
}

#[cfg(test)]
use crate::file::load_resource_str;

#[test]
#[allow(clippy::self_named_module_files)]
fn test_query() {
    let mut engine = CirupEngine::new();
    engine.register_table_from_str("A", "test.json", include_str!("../test/test.json"));
    engine.register_table_from_str("B", "test.resx", include_str!("../test/test.resx"));

    // find the union of the two tables (merge strings)
    let resources = engine.query_resource("SELECT * FROM A UNION SELECT * from B");
    print_resources_pretty(&resources);

    assert_eq!(resources.len(), 6);

    // find the intersection of the two tables (common strings)
    let resources = engine.query_resource("SELECT * FROM A INTERSECT SELECT * from B");
    print_resources_pretty(&resources);

    assert_eq!(resources.len(), 3);
}

#[test]
fn test_query_subtract() {
    let mut engine = CirupEngine::new();

    engine.register_table_from_str("A", "test1A.restext", include_str!("../test/subtract/test1A.restext"));
    engine.register_table_from_str("B", "test1B.restext", include_str!("../test/subtract/test1B.restext"));
    let expected = match load_resource_str(include_str!("../test/subtract/test1C.restext"), "restext") {
        Ok(resources) => resources,
        Err(e) => panic!("failed to parse expected restext fixture: {}", e),
    };

    let actual = engine.query_resource("SELECT * FROM A WHERE A.key NOT IN (SELECT B.key FROM B)");
    assert_eq!(actual, expected);
}

#[test]
#[allow(clippy::self_named_module_files)]
fn test_query_diff_with_base() {
    let mut engine = CirupEngine::new();
    engine.register_table_from_str("A", "test_old.resx", include_str!("../test/test_old.resx"));
    engine.register_table_from_str("B", "test_new.resx", include_str!("../test/test_new.resx"));
    engine.register_table_from_str("C", "test.resx", include_str!("../test/test.resx"));

    let triples = engine.query_triple(DIFF_WITH_BASE_QUERY);

    assert_eq!(triples.len(), 2);
    assert_eq!(triples[0].name, String::from("lblYolo"));
    assert_eq!(triples[0].base, String::from("You only live once"));
    assert_eq!(triples[0].value, String::from("Juste une vie a vivre"));
}

#[test]
#[cfg(feature = "turso-rust")]
fn test_query_turso_remote_env_gated() {
    let remote_url = std::env::var("CIRUP_TURSO_URL")
        .ok()
        .or_else(|| std::env::var("LIBSQL_URL").ok())
        .or_else(|| std::env::var("LIBSQL_HRANA_URL").ok());

    let Some(remote_url) = remote_url else {
        return;
    };

    let remote_auth_token = std::env::var("CIRUP_TURSO_AUTH_TOKEN")
        .ok()
        .or_else(|| std::env::var("LIBSQL_AUTH_TOKEN").ok())
        .or_else(|| std::env::var("TURSO_AUTH_TOKEN").ok())
        .unwrap_or_default();

    let mut query_config = QueryConfig {
        backend: QueryBackendKind::TursoRemote,
        ..QueryConfig::default()
    };
    query_config.turso.url = Some(remote_url);
    if !remote_auth_token.is_empty() {
        query_config.turso.auth_token = Some(remote_auth_token);
    }

    let mut engine = CirupEngine::with_query_config(&query_config);
    engine.register_table_from_str("A", "test.json", include_str!("../test/test.json"));

    let mut actual = engine.query_resource("SELECT * FROM A ORDER BY A.key");
    let mut expected = match load_resource_str(include_str!("../test/test.json"), "json") {
        Ok(resources) => resources,
        Err(e) => panic!("failed to parse expected json fixture: {}", e),
    };

    actual.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));
    expected.sort_by(|a, b| a.name.cmp(&b.name).then(a.value.cmp(&b.value)));

    assert_eq!(actual, expected);
}

#[test]
fn test_query_run_options_filter_and_limit_resources() {
    let query = query_print_with_backend("test.json", QueryBackendKind::Rusqlite);
    let options = QueryRunOptions {
        key_filters: vec![String::from("^lbl.*Yolo$")],
        limit: Some(1),
        ..QueryRunOptions::default()
    };

    let resources = query.run_with_options(&options);

    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].name, String::from("lblYolo"));
}

#[test]
fn test_render_resources_jsonl() {
    let resources = vec![Resource::new("hello", "world"), Resource::new("goodbye", "moon")];
    let output = render_resources(&resources, QueryOutputFormat::Jsonl);

    assert_eq!(
        output,
        "{\"name\":\"hello\",\"value\":\"world\"}\n{\"name\":\"goodbye\",\"value\":\"moon\"}\n"
    );
}

#[test]
fn test_render_resources_table_preserves_ascii_layout() {
    let resources = vec![
        Resource::new("lblBoat", "I'm on a boat."),
        Resource::new("lblYolo", "You only live once"),
        Resource::new("lblDogs", "Who let the dogs out?"),
        Resource::new("language.en", "English"),
        Resource::new("language.fr", "French"),
        Resource::new("very.deep.object", "value"),
    ];

    let output = render_resources(&resources, QueryOutputFormat::Table);
    let expected = concat!(
        "+------------------+-----------------------+\n",
        "| name             | value                 |\n",
        "+------------------+-----------------------+\n",
        "| lblBoat          | I'm on a boat.        |\n",
        "+------------------+-----------------------+\n",
        "| lblYolo          | You only live once    |\n",
        "+------------------+-----------------------+\n",
        "| lblDogs          | Who let the dogs out? |\n",
        "+------------------+-----------------------+\n",
        "| language.en      | English               |\n",
        "+------------------+-----------------------+\n",
        "| language.fr      | French                |\n",
        "+------------------+-----------------------+\n",
        "| very.deep.object | value                 |\n",
        "+------------------+-----------------------+\n",
    );

    assert_eq!(output, expected);
}

#[test]
fn test_render_triples_json() {
    let triples = vec![Triple::new("hello", "world", "base")];
    let output = render_triples(&triples, QueryOutputFormat::Json);

    assert_eq!(output, "[{\"name\":\"hello\",\"value\":\"world\",\"base\":\"base\"}]\n");
}

#[test]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_render_and_filter_resources_large_input() {
    let resources = (0..50_000usize)
        .map(|index| {
            Resource::new(
                &format!("group{index}.key{}", index % 17),
                if index % 3 == 0 { "English" } else { "French" },
            )
        })
        .collect::<Vec<_>>();

    let options = QueryRunOptions {
        key_filters: vec![String::from("^group.*")],
        value_filters: vec![String::from("^English$")],
        limit: Some(10_000),
        ..QueryRunOptions::default()
    };
    let filters = compile_query_filters(&options).expect("failed to compile benchmark filters");

    let started = Instant::now();
    let rendered = render_resources(&resources, QueryOutputFormat::Jsonl);
    let render_elapsed = started.elapsed();

    let started = Instant::now();
    let (counts, filtered) = filter_resources(resources.clone(), &filters, options.limit);
    let filter_elapsed = started.elapsed();

    assert!(!rendered.is_empty());
    assert_eq!(counts.output_count, filtered.len());

    println!(
        "query render/filter benchmark: input={} rendered_bytes={} filtered={} render={:?} filter={:?}",
        resources.len(),
        rendered.len(),
        filtered.len(),
        render_elapsed,
        filter_elapsed
    );
}

#[test]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_compile_query_filters_large_input() {
    let options = QueryRunOptions {
        key_filters: (0..5_000usize)
            .map(|index| format!("^group{index}.*value{}$", index % 23))
            .collect(),
        value_filters: (0..5_000usize)
            .map(|index| format!("^lang{}.*English$", index % 19))
            .collect(),
        ..QueryRunOptions::default()
    };

    let started = Instant::now();
    let filters = compile_query_filters(&options).expect("failed to compile benchmark filters");
    let compile_elapsed = started.elapsed();

    let started = Instant::now();
    let key_sql = filters
        .key_filter
        .as_ref()
        .expect("expected key filter")
        .sql_condition("filtered.key");
    let value_sql = filters
        .value_filter
        .as_ref()
        .expect("expected value filter")
        .sql_condition("filtered.val");
    let sql_elapsed = started.elapsed();

    assert!(!key_sql.is_empty());
    assert!(!value_sql.is_empty());

    println!(
        "query filter benchmark: key_patterns={} value_patterns={} compile={:?} sql={:?}",
        options.key_filters.len(),
        options.value_filters.len(),
        compile_elapsed,
        sql_elapsed
    );
}

#[test]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_push_changed_values_duplicate_keys() {
    let mut left = String::new();
    let mut right = String::new();

    for key_index in 0..8_000usize {
        for left_variant in 0..6usize {
            left.push_str(&format!("key{key_index}=left{left_variant}\r\n"));
        }

        right.push_str(&format!("key{key_index}=left0\r\n"));
        right.push_str(&format!("key{key_index}=right{}\r\n", key_index % 7));
    }

    let mut engine = CirupEngine::with_backend(QueryBackendKind::TursoLocal);
    engine.register_table_from_str("A", "left.restext", &left);
    engine.register_table_from_str("B", "right.restext", &right);

    let query = r"
        SELECT
            B.key, B.val
        FROM B
        INNER JOIN A on (A.key = B.key) AND (A.val <> B.val)";

    let started = Instant::now();
    let resources = engine.query_resource(query);
    let elapsed = started.elapsed();

    assert!(!resources.is_empty());

    println!(
        "push-changed-values benchmark: left_rows={} right_rows={} output_rows={} elapsed={:?}",
        left.lines().count(),
        right.lines().count(),
        resources.len(),
        elapsed
    );
}

#[test]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_pull_left_join_duplicate_keys() {
    let mut left = String::new();
    let mut right = String::new();

    for key_index in 0..8_000usize {
        for left_variant in 0..6usize {
            left.push_str(&format!("key{key_index}=left{left_variant}\r\n"));
        }

        right.push_str(&format!("key{key_index}=right{}\r\n", key_index % 7));
        right.push_str(&format!("key{key_index}=right{}\r\n", (key_index + 1) % 7));
    }

    let mut engine = CirupEngine::with_backend(QueryBackendKind::TursoLocal);
    engine.register_table_from_str("A", "left.restext", &left);
    engine.register_table_from_str("B", "right.restext", &right);

    let query = r"
        SELECT
            A.key, A.val
        FROM A
        LEFT OUTER JOIN B on A.key = B.key";

    let started = Instant::now();
    let resources = engine.query_resource(query);
    let elapsed = started.elapsed();

    assert!(!resources.is_empty());

    println!(
        "pull-left-join benchmark: left_rows={} right_rows={} output_rows={} elapsed={:?}",
        left.lines().count(),
        right.lines().count(),
        resources.len(),
        elapsed
    );
}

#[test]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_fast_query_dispatch_repeated() {
    let mut engine = CirupEngine::with_backend(QueryBackendKind::TursoLocal);
    engine.register_table_from_str("A", "empty.restext", "");

    let iterations = 200_000usize;
    let started = Instant::now();

    for _ in 0..iterations {
        let resources = engine.query_resource(PRINT_QUERY);
        assert!(resources.is_empty());
    }

    let elapsed = started.elapsed();
    println!(
        "fast-query-dispatch benchmark: iterations={} elapsed={:?}",
        iterations, elapsed
    );
}

#[test]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_render_resource_table_repeated() {
    let resources = vec![
        Resource::new("lblBoat", "I'm on a boat."),
        Resource::new("lblYolo", "You only live once"),
        Resource::new("lblDogs", "Who let the dogs out?"),
        Resource::new("language.en", "English"),
        Resource::new("language.fr", "French"),
        Resource::new("very.deep.object", "value"),
    ];

    let started = Instant::now();
    let mut total_bytes = 0usize;

    for _ in 0..20_000 {
        total_bytes += render_resources(&resources, QueryOutputFormat::Table).len();
    }

    let elapsed = started.elapsed();

    println!(
        "resource-table benchmark: iterations={} total_bytes={} elapsed={:?}",
        20_000, total_bytes, elapsed
    );
}

#[test]
#[ignore = "benchmark: run manually with --ignored --nocapture"]
#[allow(clippy::print_stdout)]
fn benchmark_render_report_table_repeated() {
    let report = QueryExecutionReport {
        operation: Some(String::from("file-merge")),
        result_kind: String::from("resource"),
        input_files: vec![String::from("a.json"), String::from("b.json"), String::from("c.json")],
        output_file: Some(String::from("out.json")),
        matched_count: 50_000,
        filtered_count: 37_500,
        output_count: 10_000,
        truncated: true,
        dry_run: false,
        check: true,
        would_write: true,
        wrote_output: false,
        change_detected: true,
    };

    let iterations = 20_000usize;
    let started = Instant::now();
    let mut total_bytes = 0usize;

    for _ in 0..iterations {
        total_bytes += render_report(&report, QueryOutputFormat::Table).len();
    }

    let elapsed = started.elapsed();
    assert!(total_bytes > 0);

    println!(
        "render-report benchmark: iterations={} total_bytes={} elapsed={:?}",
        iterations, total_bytes, elapsed
    );
}

#[test]
fn test_count_only_rejects_output_file() {
    let options = QueryRunOptions {
        count_only: true,
        ..QueryRunOptions::default()
    };

    let error = options
        .validate_for_output(Some("out.json"))
        .expect_err("expected validation error");
    assert_eq!(error.to_string(), "--count-only cannot be combined with an output file");
}

#[test]
fn test_summary_rejects_count_only() {
    let options = QueryRunOptions {
        count_only: true,
        summary: true,
        ..QueryRunOptions::default()
    };

    let error = options
        .validate_for_output(None)
        .expect_err("expected validation error");
    assert_eq!(error.to_string(), "--count-only cannot be combined with --summary");
}

#[test]
fn test_report_detects_change_for_stdout_results() {
    let report = QueryExecutionReport::from_options(
        &QueryRunOptions::default().with_context("file-diff", &["a.json", "b.json"], None),
        "resource",
        QueryExecutionCounts {
            matched_count: 3,
            filtered_count: 2,
            output_count: 2,
            truncated: false,
        },
        false,
        false,
    );

    assert!(report.indicates_change());
}

#[test]
fn test_report_renders_as_json_summary() {
    let report = QueryExecutionReport::from_options(
        &QueryRunOptions::default().with_context("file-sort", &["a.json"], Some("a.json")),
        "resource",
        QueryExecutionCounts {
            matched_count: 4,
            filtered_count: 4,
            output_count: 4,
            truncated: false,
        },
        true,
        false,
    );

    let output = render_report(&report, QueryOutputFormat::Json);

    assert!(output.contains("\"operation\":\"file-sort\""));
    assert!(output.contains("\"would_write\":true"));
    assert!(output.ends_with('\n'));
}

#[test]
fn test_report_renders_as_table_summary() {
    let report = QueryExecutionReport {
        operation: Some(String::from("file-sort")),
        result_kind: String::from("resource"),
        input_files: vec![String::from("cirup_core/test/test.json")],
        output_file: Some(String::from("cirup_core/test/test.json")),
        matched_count: 6,
        filtered_count: 6,
        output_count: 6,
        truncated: false,
        dry_run: true,
        check: false,
        would_write: true,
        wrote_output: false,
        change_detected: true,
    };

    let output = render_report(&report, QueryOutputFormat::Table);
    let expected = concat!(
        "+-----------------+---------------------------+\n",
        "| field           | value                     |\n",
        "+-----------------+---------------------------+\n",
        "| operation       | file-sort                 |\n",
        "+-----------------+---------------------------+\n",
        "| result_kind     | resource                  |\n",
        "+-----------------+---------------------------+\n",
        "| input_files     | cirup_core/test/test.json |\n",
        "+-----------------+---------------------------+\n",
        "| output_file     | cirup_core/test/test.json |\n",
        "+-----------------+---------------------------+\n",
        "| matched_count   | 6                         |\n",
        "+-----------------+---------------------------+\n",
        "| filtered_count  | 6                         |\n",
        "+-----------------+---------------------------+\n",
        "| output_count    | 6                         |\n",
        "+-----------------+---------------------------+\n",
        "| truncated       | false                     |\n",
        "+-----------------+---------------------------+\n",
        "| dry_run         | true                      |\n",
        "+-----------------+---------------------------+\n",
        "| check           | false                     |\n",
        "+-----------------+---------------------------+\n",
        "| would_write     | true                      |\n",
        "+-----------------+---------------------------+\n",
        "| wrote_output    | false                     |\n",
        "+-----------------+---------------------------+\n",
        "| change_detected | true                      |\n",
        "+-----------------+---------------------------+\n",
    );

    assert_eq!(output, expected);
}

#[test]
fn test_compile_text_pattern_supports_simple_regex_subset() {
    let compiled = compile_text_pattern("--key-filter", "^lbl.*Yolo$").expect("expected valid pattern");

    assert!(compiled.matches("lblMyYolo"));
    assert!(!compiled.matches("prefix_lblMyYolo"));
    assert_eq!(compiled.glob_pattern, "lbl*Yolo");
}

#[test]
fn test_compile_text_pattern_rejects_unsupported_syntax() {
    let error = compile_text_pattern("--key-filter", "foo|bar").expect_err("expected invalid pattern");

    assert!(error.to_string().contains("unsupported syntax '|'"));
}

#[test]
fn test_compile_text_filter_repeats_with_or_semantics() {
    let options = QueryRunOptions {
        key_filters: vec![String::from("^lbl"), String::from("World$")],
        ..QueryRunOptions::default()
    };
    let text_filter = compile_text_filter("--key-filter", &options.key_filters)
        .expect("expected valid filter")
        .expect("expected compiled patterns");

    assert!(text_filter.matches("lblHello"));
    assert!(text_filter.matches("HelloWorld"));
    assert!(!text_filter.matches("other"));
}

#[test]
fn test_wrap_resource_query_with_key_filter_uses_glob_condition() {
    let options = QueryRunOptions {
        key_filters: vec![String::from("^lbl")],
        ..QueryRunOptions::default()
    };
    let filters = compile_query_filters(&options).expect("expected compiled filters");
    let wrapped = wrap_resource_query_with_filters(PRINT_QUERY, &filters);

    assert!(wrapped.contains("filtered.key GLOB 'lbl*'"));
    assert!(wrapped.starts_with(&format!("WITH filtered(key, val) AS ({PRINT_QUERY})")));
}

#[test]
fn test_wrap_resource_query_with_value_filter_uses_glob_condition() {
    let options = QueryRunOptions {
        value_filters: vec![String::from("^Hello")],
        ..QueryRunOptions::default()
    };
    let filters = compile_query_filters(&options).expect("expected compiled filters");
    let wrapped = wrap_resource_query_with_filters(PRINT_QUERY, &filters);

    assert!(wrapped.contains("filtered.val GLOB 'Hello*'"));
}

#[test]
fn test_value_filter_matches_resource_values() {
    let query = query_print_with_backend("test.json", QueryBackendKind::Rusqlite);
    let options = QueryRunOptions {
        value_filters: vec![String::from("^English$")],
        ..QueryRunOptions::default()
    };

    let resources = query.run_with_options(&options);

    assert!(!resources.is_empty());
    assert!(resources.iter().all(|resource| resource.value == "English"));
}

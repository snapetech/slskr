#[derive(Clone, Debug)]
pub struct DotNetRegex {
    matcher: fancy_regex::Regex,
    lowercase_input: bool,
}

impl DotNetRegex {
    pub fn compile(expression: &str, case_sensitive: bool) -> Result<Self, String> {
        let conditionals = rewrite_assertion_conditionals(expression)?;
        let numbered = rewrite_dotnet_numbered_captures(&conditionals)?;
        let normalized = normalize_dotnet_pattern(&numbered)?;
        let lowercase_input = !case_sensitive && !pattern_has_inline_case_mode(&normalized);
        let normalized = if lowercase_input {
            lowercase_dotnet_pattern_literals(&normalized)
        } else {
            normalized
        };
        let matcher = fancy_regex::RegexBuilder::new(&normalized)
            .case_insensitive(!case_sensitive && !lowercase_input)
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            matcher,
            lowercase_input,
        })
    }

    pub fn validate(expression: &str) -> Result<(), String> {
        Self::compile(expression, true).map(|_| ())
    }

    pub fn is_match(&self, value: &str) -> Result<bool, String> {
        let folded;
        let value = if self.lowercase_input {
            folded = value.to_lowercase();
            folded.as_str()
        } else {
            value
        };
        self.matcher
            .is_match(value)
            .map_err(|error| error.to_string())
    }
}

fn pattern_has_inline_case_mode(expression: &str) -> bool {
    let mut index = 0;
    while index < expression.len() {
        if expression.as_bytes()[index] == b'\\' {
            index += escaped_token_len(expression, index);
            continue;
        }
        if let Some(options) = parse_inline_options(expression, index) {
            if options.case_insensitive.is_some() {
                return true;
            }
            index = options.end;
            continue;
        }
        index += expression[index..]
            .chars()
            .next()
            .expect("index is within expression")
            .len_utf8();
    }
    false
}

fn lowercase_dotnet_pattern_literals(expression: &str) -> String {
    let bytes = expression.as_bytes();
    let mut folded = String::with_capacity(expression.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            if matches!(bytes.get(index + 1), Some(b'p' | b'P'))
                && bytes.get(index + 2) == Some(&b'{')
            {
                if let Some(end) = expression[index + 3..].find('}') {
                    let end = index + 3 + end + 1;
                    folded.push_str(&expression[index..end]);
                    index = end;
                    continue;
                }
            }
            if bytes.get(index + 1) == Some(&b'k')
                && matches!(bytes.get(index + 2), Some(b'<' | b'\''))
            {
                let delimiter = if bytes[index + 2] == b'<' { '>' } else { '\'' };
                if let Some(end) = expression[index + 3..].find(delimiter) {
                    let end = index + 3 + end + 1;
                    folded.push_str(&expression[index..end]);
                    index = end;
                    continue;
                }
            }
            let length = escaped_token_len(expression, index);
            folded.push_str(&expression[index..index + length]);
            index += length;
            continue;
        }
        let remaining = &expression[index..];
        if remaining.starts_with("(?#") {
            if let Some(end) = remaining.find(')') {
                folded.push_str(&remaining[..=end]);
                index += end + 1;
                continue;
            }
        }
        if remaining.starts_with("(?<")
            && !remaining.starts_with("(?<=")
            && !remaining.starts_with("(?<!")
        {
            if let Some(end) = remaining.find('>') {
                folded.push_str(&remaining[..=end]);
                index += end + 1;
                continue;
            }
        }
        if let Some(named) = remaining.strip_prefix("(?'") {
            if let Some(end) = named.find('\'') {
                let end = 3 + end;
                folded.push_str(&remaining[..=end]);
                index += end + 1;
                continue;
            }
        }
        if remaining.starts_with("(?(") {
            if let Some(end) = remaining.find(')') {
                folded.push_str(&remaining[..=end]);
                index += end + 1;
                continue;
            }
        }
        if let Some(options) = parse_inline_options(expression, index) {
            folded.push_str(&expression[index..options.end]);
            index = options.end;
            continue;
        }
        let character = expression[index..]
            .chars()
            .next()
            .expect("index is within expression");
        folded.extend(character.to_lowercase());
        index += character.len_utf8();
    }
    folded
}

fn rewrite_dotnet_numbered_captures(expression: &str) -> Result<String, String> {
    let bytes = expression.as_bytes();
    let mut unnamed = Vec::new();
    let mut named = Vec::<String>::new();
    let mut numeric_named = Vec::<(usize, usize)>::new();
    let mut index = 0;
    let mut class_depth = 0_u32;
    let mut mode = RegexMode::default();
    let mut group_modes = Vec::new();

    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += escaped_token_len(expression, index);
            continue;
        }
        if class_depth == 0 {
            let remaining = &expression[index..];
            if remaining.starts_with("(?#") {
                let Some(end) = remaining.find(')') else {
                    return Err("unterminated .NET regular-expression comment".to_owned());
                };
                index += end + 1;
                continue;
            }
            if bytes[index] == b'(' {
                if let Some(options) = parse_inline_options(expression, index) {
                    if options.scoped {
                        group_modes.push(mode);
                    }
                    if let Some(enabled) = options.explicit_capture {
                        mode.explicit_capture = enabled;
                    }
                    if let Some(enabled) = options.multiline {
                        mode.multiline = enabled;
                    }
                    index = options.end;
                    continue;
                }
                group_modes.push(mode);
                let conditional_target = index >= 2 && &bytes[index - 2..index] == b"(?";
                if let Some(name) = dotnet_named_capture(remaining) {
                    if let Ok(slot) = name.parse::<usize>() {
                        if slot > 0 {
                            numeric_named.push((index, slot));
                        }
                    }
                    if !name.is_empty() && !named.contains(&name) {
                        named.push(name);
                    }
                } else if !mode.explicit_capture
                    && !remaining.starts_with("(?")
                    && !conditional_target
                {
                    unnamed.push(index);
                }
            } else if bytes[index] == b')' {
                if let Some(previous) = group_modes.pop() {
                    mode = previous;
                }
            } else if bytes[index] == b'[' {
                class_depth = 1;
            }
        } else if bytes[index] == b'[' {
            class_depth += 1;
        } else if bytes[index] == b']' {
            class_depth -= 1;
        }
        index += expression[index..]
            .chars()
            .next()
            .expect("index is within expression")
            .len_utf8();
    }

    if named.is_empty() {
        return Ok(expression.to_owned());
    }

    let explicit_slots = named
        .iter()
        .filter_map(|name| name.parse::<usize>().ok())
        .filter(|slot| *slot > 0)
        .collect::<std::collections::BTreeSet<_>>();
    let mut unnamed_slots = Vec::with_capacity(unnamed.len());
    let mut next_slot = 1;
    for _ in &unnamed {
        while explicit_slots.contains(&next_slot) {
            next_slot += 1;
        }
        unnamed_slots.push(next_slot);
        next_slot += 1;
    }
    let mut next_named_slot = unnamed_slots
        .iter()
        .chain(explicit_slots.iter())
        .copied()
        .max()
        .unwrap_or(0)
        + 1;
    let mut named_slots = Vec::with_capacity(named.len());
    for name in &named {
        let slot = name
            .parse::<usize>()
            .ok()
            .filter(|slot| *slot > 0)
            .unwrap_or_else(|| {
                while explicit_slots.contains(&next_named_slot) {
                    next_named_slot += 1;
                }
                let slot = next_named_slot;
                next_named_slot += 1;
                slot
            });
        named_slots.push((slot, name.clone()));
    }
    let maximum_slot = unnamed_slots
        .iter()
        .copied()
        .chain(named_slots.iter().map(|(slot, _)| *slot))
        .max()
        .unwrap_or(0);
    let mut targets = vec![String::new(); maximum_slot + 1];
    for slot in &unnamed_slots {
        targets[*slot] = format!("slskrDotNetCapture{slot}");
    }
    for (slot, name) in named_slots {
        targets[slot] = if name.parse::<usize>().is_ok() {
            format!("slskrDotNetCapture{slot}")
        } else {
            name
        };
    }

    let mut replacements = unnamed
        .into_iter()
        .zip(unnamed_slots)
        .map(|(position, slot)| (position, (1, format!("(?<{}>", targets[slot]))))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (position, slot) in numeric_named {
        let remaining = &expression[position..];
        let delimiter = if remaining.starts_with("(?<") {
            '>'
        } else {
            '\''
        };
        let consumed = remaining.find(delimiter).map_or(1, |end| end + 1);
        replacements.insert(position, (consumed, format!("(?<{}>", targets[slot])));
    }
    let mut rewritten = String::with_capacity(expression.len() + replacements.len() * 24);
    index = 0;
    class_depth = 0;
    while index < bytes.len() {
        if let Some((consumed, replacement)) = replacements.get(&index) {
            rewritten.push_str(replacement);
            index += *consumed;
            continue;
        }
        if class_depth == 0 && expression[index..].starts_with("(?(") {
            let target_start = index + 3;
            let mut target_end = target_start;
            while bytes.get(target_end).is_some_and(u8::is_ascii_digit) {
                target_end += 1;
            }
            if target_end > target_start && bytes.get(target_end) == Some(&b')') {
                if let Ok(slot) = expression[target_start..target_end].parse::<usize>() {
                    if let Some(target) = targets.get(slot).filter(|target| !target.is_empty()) {
                        rewritten.push_str("(?(<");
                        rewritten.push_str(target);
                        rewritten.push_str(">)");
                        index = target_end + 1;
                        continue;
                    }
                }
            }
        }
        if bytes[index] == b'\\' {
            if class_depth == 0 && bytes.get(index + 1).is_some_and(u8::is_ascii_digit) {
                let mut end = index + 1;
                while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                    end += 1;
                }
                if let Ok(slot) = expression[index + 1..end].parse::<usize>() {
                    if let Some(target) = targets.get(slot).filter(|target| !target.is_empty()) {
                        rewritten.push_str("\\k<");
                        rewritten.push_str(target);
                        rewritten.push('>');
                        index = end;
                        continue;
                    }
                }
            }
            let length = escaped_token_len(expression, index);
            rewritten.push_str(&expression[index..index + length]);
            index += length;
            continue;
        }
        if bytes[index] == b'[' {
            class_depth += 1;
        } else if bytes[index] == b']' && class_depth > 0 {
            class_depth -= 1;
        }
        let character = expression[index..]
            .chars()
            .next()
            .expect("index is within expression");
        rewritten.push(character);
        index += character.len_utf8();
    }
    Ok(rewritten)
}

fn escaped_token_len(expression: &str, index: usize) -> usize {
    expression[index + 1..]
        .chars()
        .next()
        .map_or(1, |character| 1 + character.len_utf8())
}

fn dotnet_named_capture(expression: &str) -> Option<String> {
    let (start, delimiter) = if expression.starts_with("(?<")
        && !expression.starts_with("(?<=")
        && !expression.starts_with("(?<!")
    {
        (3, '>')
    } else if expression.starts_with("(?'") {
        (3, '\'')
    } else {
        return None;
    };
    let end = expression[start..].find(delimiter)? + start;
    let specification = &expression[start..end];
    Some(
        specification
            .split_once('-')
            .map_or(specification, |(capture, _)| capture)
            .to_owned(),
    )
}

fn rewrite_assertion_conditionals(expression: &str) -> Result<String, String> {
    let bytes = expression.as_bytes();
    let mut rewritten = String::with_capacity(expression.len());
    let mut index = 0;
    let mut class_depth = 0_u32;

    while index < bytes.len() {
        if bytes[index] == b'\\' {
            let character = expression[index..]
                .chars()
                .nth(1)
                .map_or(1, |character| 1 + character.len_utf8());
            rewritten.push_str(&expression[index..index + character]);
            index += character;
            continue;
        }

        if class_depth == 0 {
            if let Some((assertion, inverse)) = assertion_conditional_prefix(&expression[index..]) {
                let assertion_start = index + 2;
                let assertion_end = find_matching_parenthesis(expression, assertion_start)?;
                let conditional_end = find_matching_parenthesis(expression, index)?;
                if assertion_end >= conditional_end {
                    return Err("invalid .NET assertion conditional".to_owned());
                }
                let branches_start = assertion_end + 1;
                let alternative =
                    find_top_level_alternation(expression, branches_start, conditional_end)?;
                let true_end = alternative.unwrap_or(conditional_end);
                let false_start = alternative.map_or(conditional_end, |position| position + 1);
                let condition_body_start = assertion_start + assertion.len();
                let condition_body = &expression[condition_body_start..assertion_end];
                let true_branch =
                    rewrite_assertion_conditionals(&expression[branches_start..true_end])?;
                let false_branch =
                    rewrite_assertion_conditionals(&expression[false_start..conditional_end])?;
                let condition_body = rewrite_assertion_conditionals(condition_body)?;
                rewritten.push_str("(?:");
                rewritten.push_str(assertion);
                rewritten.push_str(&condition_body);
                rewritten.push(')');
                rewritten.push_str(&true_branch);
                rewritten.push('|');
                rewritten.push_str(inverse);
                rewritten.push_str(&condition_body);
                rewritten.push(')');
                rewritten.push_str(&false_branch);
                rewritten.push(')');
                index = conditional_end + 1;
                continue;
            }

            if bytes[index] == b'[' {
                class_depth = 1;
            }
        } else if bytes[index] == b'[' {
            class_depth += 1;
        } else if bytes[index] == b']' {
            class_depth -= 1;
        }

        let character = expression[index..]
            .chars()
            .next()
            .expect("index is within expression");
        rewritten.push(character);
        index += character.len_utf8();
    }

    Ok(rewritten)
}

fn assertion_conditional_prefix(expression: &str) -> Option<(&'static str, &'static str)> {
    if expression.starts_with("(?(?<=") {
        Some(("(?<=", "(?<!"))
    } else if expression.starts_with("(?(?<!") {
        Some(("(?<!", "(?<="))
    } else if expression.starts_with("(?(?=") {
        Some(("(?=", "(?!"))
    } else if expression.starts_with("(?(?!") {
        Some(("(?!", "(?="))
    } else {
        None
    }
}

fn find_matching_parenthesis(expression: &str, open: usize) -> Result<usize, String> {
    let bytes = expression.as_bytes();
    if bytes.get(open) != Some(&b'(') {
        return Err("invalid .NET assertion conditional".to_owned());
    }
    let mut index = open + 1;
    let mut depth = 1_u32;
    let mut class_depth = 0_u32;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index += 1;
            if index < bytes.len() {
                let character = expression[index..]
                    .chars()
                    .next()
                    .expect("index is within expression");
                index += character.len_utf8();
            }
            continue;
        }
        if class_depth == 0 {
            if expression[index..].starts_with("(?#") {
                let Some(end) = expression[index + 3..].find(')') else {
                    return Err("unterminated .NET regular-expression comment".to_owned());
                };
                index += 3 + end + 1;
                continue;
            }
            match bytes[index] {
                b'[' => class_depth = 1,
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(index);
                    }
                }
                _ => {}
            }
        } else if bytes[index] == b'[' {
            class_depth += 1;
        } else if bytes[index] == b']' {
            class_depth -= 1;
        }
        let character = expression[index..]
            .chars()
            .next()
            .expect("index is within expression");
        index += character.len_utf8();
    }
    Err("unterminated .NET assertion conditional".to_owned())
}

fn find_top_level_alternation(
    expression: &str,
    start: usize,
    end: usize,
) -> Result<Option<usize>, String> {
    let bytes = expression.as_bytes();
    let mut index = start;
    let mut parenthesis_depth = 0_u32;
    let mut class_depth = 0_u32;
    let mut alternative = None;
    while index < end {
        if bytes[index] == b'\\' {
            index += 1;
            if index < end {
                let character = expression[index..]
                    .chars()
                    .next()
                    .expect("index is within expression");
                index += character.len_utf8();
            }
            continue;
        }
        if class_depth == 0 {
            match bytes[index] {
                b'[' => class_depth = 1,
                b'(' => parenthesis_depth += 1,
                b')' => parenthesis_depth = parenthesis_depth.saturating_sub(1),
                b'|' if parenthesis_depth == 0 && alternative.replace(index).is_some() => {
                    return Err(
                        ".NET assertion conditionals allow at most one alternation".to_owned()
                    );
                }
                _ => {}
            }
        } else if bytes[index] == b'[' {
            class_depth += 1;
        } else if bytes[index] == b']' {
            class_depth -= 1;
        }
        let character = expression[index..]
            .chars()
            .next()
            .expect("index is within expression");
        index += character.len_utf8();
    }
    Ok(alternative)
}

fn normalize_dotnet_pattern(expression: &str) -> Result<String, String> {
    let bytes = expression.as_bytes();
    let mut normalized = String::with_capacity(expression.len());
    let mut index = 0;
    let mut class_depth = 0_u32;
    let mut mode = RegexMode::default();
    let mut group_modes = Vec::new();

    while index < bytes.len() {
        if bytes[index] == b'\\' {
            let Some(&escaped) = bytes.get(index + 1) else {
                normalized.push('\\');
                index += 1;
                continue;
            };

            if matches!(escaped, b'p' | b'P') && bytes.get(index + 2) == Some(&b'{') {
                let Some(relative_end) = expression[index + 3..].find('}') else {
                    return Err("unterminated .NET Unicode property escape".to_owned());
                };
                let end = index + 3 + relative_end;
                let property = &expression[index + 3..end];
                if !dotnet_unicode_property_name(property) {
                    return Err(format!(
                        "Unicode property {property:?} is not supported by .NET Regex"
                    ));
                }
                if property.eq_ignore_ascii_case("IsBasicLatin") {
                    if escaped == b'P' {
                        normalized.push_str("[^\\x00-\\x7F]");
                    } else {
                        normalized.push_str("[\\x00-\\x7F]");
                    }
                } else {
                    normalized.push_str(&expression[index..=end]);
                }
                index = end + 1;
                continue;
            }
            if matches!(escaped, b'H' | b'K' | b'R' | b'X' | b'g' | b'h') {
                return Err(format!(
                    "escape \\{} is not supported by .NET Regex",
                    char::from(escaped)
                ));
            }
            if escaped == b'x' && bytes.get(index + 2) == Some(&b'{') {
                return Err("braced hexadecimal escapes are not supported by .NET Regex".to_owned());
            }
            if escaped == b'c' {
                let Some(&control) = bytes.get(index + 2) else {
                    return Err("incomplete .NET control-character escape".to_owned());
                };
                if !control.is_ascii_alphabetic() {
                    return Err("invalid .NET control-character escape".to_owned());
                }
                let value = control.to_ascii_uppercase() - b'A' + 1;
                normalized.push_str(&format!("\\x{value:02X}"));
                index += 3;
                continue;
            }
            if class_depth == 0 && escaped == b'Z' {
                normalized.push_str("(?=\\n?\\z)");
                index += 2;
                continue;
            }

            normalized.push('\\');
            normalized.push(char::from(escaped));
            index += 2;
            continue;
        }

        if class_depth == 0 {
            let remaining = &expression[index..];
            if remaining.starts_with("(?~")
                || remaining.starts_with("(?R)")
                || remaining.starts_with("(?0)")
                || remaining.starts_with("(?P")
                || remaining.starts_with("(?|")
                || remaining.starts_with("(*")
            {
                return Err("regular expression uses syntax not supported by .NET Regex".to_owned());
            }

            if bytes[index] == b'+'
                && index > 0
                && (matches!(bytes[index - 1], b'?' | b'*' | b'+')
                    || bytes[index - 1] == b'}' && preceding_braced_quantifier(expression, index))
            {
                return Err("possessive quantifiers are not supported by .NET Regex".to_owned());
            }

            if bytes[index] == b'$' && !mode.multiline {
                normalized.push_str("(?=\\n?\\z)");
                index += 1;
                continue;
            }

            if remaining.starts_with("(?#") {
                let Some(end) = remaining.find(')') else {
                    return Err("unterminated .NET regular-expression comment".to_owned());
                };
                normalized.push_str(&remaining[..=end]);
                index += end + 1;
                continue;
            }

            if bytes[index] == b'(' {
                if let Some(options) = parse_inline_options(expression, index) {
                    if options.scoped {
                        group_modes.push(mode);
                    }
                    if let Some(enabled) = options.explicit_capture {
                        mode.explicit_capture = enabled;
                    }
                    if let Some(enabled) = options.multiline {
                        mode.multiline = enabled;
                    }
                    normalized.push_str(&options.rendered);
                    index = options.end;
                    continue;
                }

                group_modes.push(mode);
                let conditional_target = index >= 2 && &bytes[index - 2..index] == b"(?";
                if mode.explicit_capture && !remaining.starts_with("(?") && !conditional_target {
                    normalized.push_str("(?:");
                    index += 1;
                    continue;
                }
            } else if bytes[index] == b')' {
                if let Some(previous) = group_modes.pop() {
                    mode = previous;
                }
            }

            if bytes[index] == b'[' {
                class_depth = 1;
            }
        } else if bytes[index] == b'[' {
            class_depth += 1;
        } else if bytes[index] == b']' {
            class_depth -= 1;
        }

        let character = expression[index..]
            .chars()
            .next()
            .expect("index is within expression");
        normalized.push(character);
        index += character.len_utf8();
    }

    Ok(normalized)
}

fn preceding_braced_quantifier(expression: &str, suffix: usize) -> bool {
    let prefix = &expression[..suffix - 1];
    let Some(open) = prefix.rfind('{') else {
        return false;
    };
    let body = &expression[open + 1..suffix - 1];
    !body.is_empty()
        && body
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b',')
        && body.bytes().any(|byte| byte.is_ascii_digit())
}

fn dotnet_unicode_property_name(property: &str) -> bool {
    matches!(
        property,
        "L" | "Lu"
            | "Ll"
            | "Lt"
            | "Lm"
            | "Lo"
            | "M"
            | "Mn"
            | "Mc"
            | "Me"
            | "N"
            | "Nd"
            | "Nl"
            | "No"
            | "P"
            | "Pc"
            | "Pd"
            | "Ps"
            | "Pe"
            | "Pi"
            | "Pf"
            | "Po"
            | "S"
            | "Sm"
            | "Sc"
            | "Sk"
            | "So"
            | "Z"
            | "Zs"
            | "Zl"
            | "Zp"
            | "C"
            | "Cc"
            | "Cf"
            | "Cs"
            | "Co"
            | "Cn"
    ) || property.starts_with("Is")
}

#[derive(Clone, Copy, Default)]
struct RegexMode {
    explicit_capture: bool,
    multiline: bool,
}

struct InlineOptions {
    end: usize,
    scoped: bool,
    explicit_capture: Option<bool>,
    multiline: Option<bool>,
    case_insensitive: Option<bool>,
    rendered: String,
}

fn parse_inline_options(expression: &str, start: usize) -> Option<InlineOptions> {
    let bytes = expression.as_bytes();
    if bytes.get(start..start + 2) != Some(b"(?") {
        return None;
    }

    let mut index = start + 2;
    let mut disabling = false;
    let mut saw_flag = false;
    let mut enabled = String::new();
    let mut disabled = String::new();
    let mut explicit_capture = None;
    let mut multiline = None;
    let mut case_insensitive = None;

    loop {
        let flag = *bytes.get(index)?;
        match flag {
            b'i' | b'm' | b'n' | b's' | b'x' => {
                saw_flag = true;
                if flag == b'n' {
                    explicit_capture = Some(!disabling);
                } else {
                    if flag == b'm' {
                        multiline = Some(!disabling);
                    } else if flag == b'i' {
                        case_insensitive = Some(!disabling);
                    }
                    if disabling {
                        disabled.push(char::from(flag));
                    } else {
                        enabled.push(char::from(flag));
                    }
                }
                index += 1;
            }
            b'-' if !disabling => {
                disabling = true;
                index += 1;
            }
            b')' | b':' if saw_flag => {
                let scoped = flag == b':';
                let rendered = if enabled.is_empty() && disabled.is_empty() {
                    if scoped {
                        "(?:".to_owned()
                    } else {
                        String::new()
                    }
                } else {
                    let mut rendered = String::from("(?");
                    rendered.push_str(&enabled);
                    if !disabled.is_empty() {
                        rendered.push('-');
                        rendered.push_str(&disabled);
                    }
                    rendered.push(char::from(flag));
                    rendered
                };
                return Some(InlineOptions {
                    end: index + 1,
                    scoped,
                    explicit_capture,
                    multiline,
                    case_insensitive,
                    rendered,
                });
            }
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DotNetRegex;

    #[test]
    fn translates_dotnet_control_escape_and_end_anchor() {
        let control = DotNetRegex::compile(r"^\cA$", true).expect("control escape");
        assert!(control.is_match("\u{0001}").expect("control match"));

        let end = DotNetRegex::compile(r"abc\Z", true).expect("end anchor");
        assert!(end.is_match("abc").expect("absolute end"));
        assert!(end.is_match("abc\n").expect("single final newline"));
        assert!(!end.is_match("abc\n\n").expect("multiple final newlines"));

        let dollar = DotNetRegex::compile(r"abc$", true).expect("dollar anchor");
        assert!(dollar.is_match("abc\n").expect("single final newline"));
        assert!(!dollar.is_match("abc\n\n").expect("multiple final newlines"));
        assert!(!dollar.is_match("abc\r\n").expect("CRLF before dollar"));

        let multiline = DotNetRegex::compile(r"(?m)^abc$", true).expect("multiline anchor");
        assert!(multiline
            .is_match("prefix\nabc\nsuffix")
            .expect("multiline match"));
    }

    #[test]
    fn translates_dotnet_explicit_capture_mode() {
        let global = DotNetRegex::compile(r"(?n)^(a)(?<b>b)\k<b>$", true)
            .expect("global explicit-capture mode");
        assert!(global.is_match("abb").expect("global explicit capture"));

        let scoped = DotNetRegex::compile(r"^(?n:(a)(?<b>b)\k<b>)(c)$", true)
            .expect("scoped explicit-capture mode");
        assert!(scoped.is_match("abbc").expect("scoped explicit capture"));

        let disabled = DotNetRegex::compile(r"(?n)^(?-n:(a))\1$", true)
            .expect("disabled explicit-capture scope");
        assert!(disabled.is_match("aa").expect("disabled explicit capture"));
    }

    #[test]
    fn translates_dotnet_assertion_conditionals() {
        let lookahead =
            DotNetRegex::compile(r"^(?(?=a)a|b)$", true).expect("lookahead conditional");
        assert!(lookahead.is_match("a").expect("true lookahead branch"));
        assert!(lookahead.is_match("b").expect("false lookahead branch"));
        assert!(!lookahead.is_match("c").expect("no lookahead branch"));

        let lookbehind =
            DotNetRegex::compile(r"^a(?(?<=a)b|c)$", true).expect("lookbehind conditional");
        assert!(lookbehind.is_match("ab").expect("true lookbehind branch"));
        assert!(!lookbehind.is_match("ac").expect("false lookbehind branch"));
    }

    #[test]
    fn remaps_dotnet_capture_numbering_with_named_groups() {
        let matcher =
            DotNetRegex::compile(r"^(?<named>a)(b)\1\2$", true).expect("mixed capture numbering");
        assert!(matcher.is_match("abba").expect(".NET capture order"));
        assert!(!matcher
            .is_match("abab")
            .expect("encounter-order capture mismatch"));

        let duplicate = DotNetRegex::compile(r"^(?<N>a)(?<N>b)\1$", true)
            .expect("duplicate named capture numbering");
        assert!(duplicate
            .is_match("abb")
            .expect("duplicate named capture slot"));

        let numeric_name =
            DotNetRegex::compile(r"^(?<2>a)(b)\1\2$", true).expect("explicit numeric capture name");
        assert!(numeric_name
            .is_match("abba")
            .expect("explicit numeric capture slot"));

        let conditional = DotNetRegex::compile(r"^(?<named>a)?(b)(?(1)c|d)$", true)
            .expect("mixed capture conditional numbering");
        assert!(conditional
            .is_match("bc")
            .expect(".NET conditional capture slot"));
        assert!(!conditional
            .is_match("bd")
            .expect("encounter-order conditional mismatch"));
    }

    #[test]
    fn uses_dotnet_invariant_case_folding() {
        for (expression, value, expected) in [
            (r"^i$", "İ", false),
            (r"^i$", "ı", false),
            (r"^k$", "K", true),
            (r"^s$", "ſ", false),
            (r"^ß$", "ẞ", true),
            (r"^σ$", "ς", false),
            (r"^[a-z]$", "K", true),
            (r"^(?<word>S)\k<word>$", "Ss", true),
        ] {
            let matcher = DotNetRegex::compile(expression, false)
                .unwrap_or_else(|error| panic!("failed to compile {expression:?}: {error}"));
            assert_eq!(
                matcher.is_match(value).expect("case-fold match"),
                expected,
                "unexpected invariant case fold for {expression:?} against {value:?}"
            );
        }
    }

    #[test]
    fn rejects_constructs_that_dotnet_rejects() {
        for expression in [
            r"\x{41}",
            r"a++",
            r"a\Kb",
            r"(?R)",
            r"(*FAIL)",
            r"(?~a)",
            r"(?P<name>a)",
            r"(?|a|b)",
            r"\p{Letter}",
            r"\p{Greek}",
            r"\h",
            r"\H",
        ] {
            assert!(
                DotNetRegex::validate(expression).is_err(),
                "accepted non-.NET expression {expression:?}"
            );
        }
    }

    #[test]
    fn supports_dotnet_unicode_categories_and_basic_latin_block() {
        let letter = DotNetRegex::compile(r"^\p{L}+$", true).expect("letter category");
        assert!(letter.is_match("Aα").expect("letter category match"));

        let basic = DotNetRegex::compile(r"^\p{IsBasicLatin}+$", true).expect("Basic Latin block");
        assert!(basic.is_match("Az09").expect("Basic Latin match"));
        assert!(!basic.is_match("α").expect("outside Basic Latin"));

        let not_basic =
            DotNetRegex::compile(r"^\P{IsBasicLatin}+$", true).expect("negated Basic Latin block");
        assert!(not_basic.is_match("α").expect("outside Basic Latin"));
        assert!(!not_basic.is_match("A").expect("inside Basic Latin"));
    }
}

use crate::fixtures::{Fixture, FixtureConfig};
use crate::types::{AgentLabels, AgentProfile, AgentRuntimeState, RoutingRequest};
use std::fs;
use std::path::Path;

const FORMAT_MAGIC: &str = "qtom-golden-fixture-v1";

#[derive(Clone, Debug, PartialEq)]
pub struct GoldenFixture {
    pub config: FixtureConfig,
    pub fixture: Fixture,
}

#[derive(Debug)]
pub enum GoldenFixtureError {
    Io(std::io::Error),
    Format { line: usize, message: String },
}

impl std::fmt::Display for GoldenFixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Format { line, message } => {
                write!(f, "golden fixture parse error on line {line}: {message}")
            }
        }
    }
}

impl std::error::Error for GoldenFixtureError {}

impl From<std::io::Error> for GoldenFixtureError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn write_golden_fixture(
    path: impl AsRef<Path>,
    config: FixtureConfig,
    fixture: &Fixture,
) -> Result<(), GoldenFixtureError> {
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, encode_golden_fixture(config, fixture)?)?;
    Ok(())
}

pub fn read_golden_fixture(path: impl AsRef<Path>) -> Result<GoldenFixture, GoldenFixtureError> {
    let contents = fs::read_to_string(path)?;
    decode_golden_fixture(&contents)
}

fn encode_golden_fixture(
    config: FixtureConfig,
    fixture: &Fixture,
) -> Result<String, GoldenFixtureError> {
    validate_fixture_shape(config, fixture)?;

    let mut out = String::new();
    out.push_str(FORMAT_MAGIC);
    out.push('\n');
    out.push_str(&format!(
        "config {} {} {} {} {:016x}\n",
        config.agent_count, config.task_count, config.dimensions, config.k, config.seed
    ));

    out.push_str(&format!("agents {}\n", fixture.agents.len()));
    for agent in &fixture.agents {
        out.push_str(&format!(
            "agent {} {} {} {} {} {} {}",
            agent.id,
            agent.labels.model_profile,
            agent.labels.tool_profile,
            agent.labels.mcp_profile,
            agent.labels.memory_profile,
            agent.labels.cost_class,
            agent.labels.latency_class
        ));
        push_f32_bits(&mut out, &agent.vector);
        out.push('\n');
    }

    out.push_str(&format!("states {}\n", fixture.states.len()));
    for state in &fixture.states {
        out.push_str(&format!(
            "state {:08x} {:08x} {:08x} {}\n",
            state.queue_depth_norm.to_bits(),
            state.latency_norm.to_bits(),
            state.cache_pressure_norm.to_bits(),
            state.availability
        ));
    }

    out.push_str(&format!("requests {}\n", fixture.requests.len()));
    for request in &fixture.requests {
        out.push_str(&format!(
            "request {} {} {} {:08x}",
            request.task_id,
            request.k,
            request.fallback_generalist_id,
            request.radius_max_threshold.to_bits()
        ));
        push_f32_bits(&mut out, &request.vector);
        out.push('\n');
    }

    out.push_str("end\n");
    Ok(out)
}

fn decode_golden_fixture(contents: &str) -> Result<GoldenFixture, GoldenFixtureError> {
    let mut lines = NumberedLines::new(contents);
    let (line_no, magic) = lines.next_required("format magic")?;
    if magic.trim() != FORMAT_MAGIC {
        return format_error(line_no, format!("expected {FORMAT_MAGIC}"));
    }

    let (line_no, config_line) = lines.next_required("config")?;
    let config_parts = split_exact(config_line, 6, line_no, "config")?;
    expect_keyword(config_parts[0], "config", line_no)?;
    let config = FixtureConfig {
        agent_count: parse_usize(config_parts[1], line_no, "agent_count")?,
        task_count: parse_usize(config_parts[2], line_no, "task_count")?,
        dimensions: parse_usize(config_parts[3], line_no, "dimensions")?,
        k: parse_usize(config_parts[4], line_no, "k")?,
        seed: parse_hex_u64(config_parts[5], line_no, "seed")?,
    };

    let (line_no, agents_header) = lines.next_required("agents header")?;
    let agent_count = parse_count_header(agents_header, "agents", line_no)?;
    let mut agents = Vec::with_capacity(agent_count);
    for _ in 0..agent_count {
        let (line_no, line) = lines.next_required("agent")?;
        let parts = split_exact(line, 8 + config.dimensions, line_no, "agent")?;
        expect_keyword(parts[0], "agent", line_no)?;
        agents.push(AgentProfile {
            id: parse_u32(parts[1], line_no, "agent id")?,
            labels: AgentLabels {
                model_profile: parse_u16(parts[2], line_no, "model_profile")?,
                tool_profile: parse_u16(parts[3], line_no, "tool_profile")?,
                mcp_profile: parse_u16(parts[4], line_no, "mcp_profile")?,
                memory_profile: parse_u16(parts[5], line_no, "memory_profile")?,
                cost_class: parse_u8(parts[6], line_no, "cost_class")?,
                latency_class: parse_u8(parts[7], line_no, "latency_class")?,
            },
            vector: parse_f32_vec(&parts[8..], line_no, "agent vector")?,
        });
    }

    let (line_no, states_header) = lines.next_required("states header")?;
    let state_count = parse_count_header(states_header, "states", line_no)?;
    let mut states = Vec::with_capacity(state_count);
    for _ in 0..state_count {
        let (line_no, line) = lines.next_required("state")?;
        let parts = split_exact(line, 5, line_no, "state")?;
        expect_keyword(parts[0], "state", line_no)?;
        states.push(AgentRuntimeState {
            queue_depth_norm: parse_f32_bits(parts[1], line_no, "queue_depth_norm")?,
            latency_norm: parse_f32_bits(parts[2], line_no, "latency_norm")?,
            cache_pressure_norm: parse_f32_bits(parts[3], line_no, "cache_pressure_norm")?,
            availability: parse_u32(parts[4], line_no, "availability")?,
        });
    }

    let (line_no, requests_header) = lines.next_required("requests header")?;
    let request_count = parse_count_header(requests_header, "requests", line_no)?;
    let mut requests = Vec::with_capacity(request_count);
    for _ in 0..request_count {
        let (line_no, line) = lines.next_required("request")?;
        let parts = split_exact(line, 5 + config.dimensions, line_no, "request")?;
        expect_keyword(parts[0], "request", line_no)?;
        requests.push(RoutingRequest {
            task_id: parse_u64(parts[1], line_no, "task_id")?,
            k: parse_usize(parts[2], line_no, "request k")?,
            fallback_generalist_id: parse_u32(parts[3], line_no, "fallback_generalist_id")?,
            radius_max_threshold: parse_f32_bits(parts[4], line_no, "radius_max_threshold")?,
            vector: parse_f32_vec(&parts[5..], line_no, "request vector")?,
        });
    }

    let (line_no, end) = lines.next_required("end marker")?;
    if end.trim() != "end" {
        return format_error(line_no, "expected end marker");
    }
    if let Some((line_no, _)) = lines.next_nonempty() {
        return format_error(line_no, "unexpected trailing content");
    }

    let fixture = Fixture {
        agents,
        states,
        requests,
    };
    validate_fixture_shape(config, &fixture)?;

    Ok(GoldenFixture { config, fixture })
}

fn validate_fixture_shape(
    config: FixtureConfig,
    fixture: &Fixture,
) -> Result<(), GoldenFixtureError> {
    if fixture.agents.len() != config.agent_count {
        return format_error(
            0,
            format!(
                "agent count mismatch: config={}, fixture={}",
                config.agent_count,
                fixture.agents.len()
            ),
        );
    }
    if fixture.states.len() != config.agent_count {
        return format_error(
            0,
            format!(
                "state count mismatch: config={}, fixture={}",
                config.agent_count,
                fixture.states.len()
            ),
        );
    }
    if fixture.requests.len() != config.task_count {
        return format_error(
            0,
            format!(
                "request count mismatch: config={}, fixture={}",
                config.task_count,
                fixture.requests.len()
            ),
        );
    }

    for agent in &fixture.agents {
        if agent.vector.len() != config.dimensions {
            return format_error(
                0,
                format!(
                    "agent {} dimension mismatch: expected {}, got {}",
                    agent.id,
                    config.dimensions,
                    agent.vector.len()
                ),
            );
        }
    }
    for request in &fixture.requests {
        if request.vector.len() != config.dimensions {
            return format_error(
                0,
                format!(
                    "request {} dimension mismatch: expected {}, got {}",
                    request.task_id,
                    config.dimensions,
                    request.vector.len()
                ),
            );
        }
    }

    Ok(())
}

fn push_f32_bits(out: &mut String, values: &[f32]) {
    for value in values {
        out.push(' ');
        out.push_str(&format!("{:08x}", value.to_bits()));
    }
}

fn parse_count_header(
    line: &str,
    keyword: &'static str,
    line_no: usize,
) -> Result<usize, GoldenFixtureError> {
    let parts = split_exact(line, 2, line_no, keyword)?;
    expect_keyword(parts[0], keyword, line_no)?;
    parse_usize(parts[1], line_no, keyword)
}

fn split_exact<'a>(
    line: &'a str,
    expected: usize,
    line_no: usize,
    context: &'static str,
) -> Result<Vec<&'a str>, GoldenFixtureError> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() != expected {
        return format_error(
            line_no,
            format!("{context} expected {expected} fields, got {}", parts.len()),
        );
    }
    Ok(parts)
}

fn expect_keyword(
    found: &str,
    expected: &'static str,
    line_no: usize,
) -> Result<(), GoldenFixtureError> {
    if found == expected {
        Ok(())
    } else {
        format_error(line_no, format!("expected keyword {expected}, got {found}"))
    }
}

fn parse_u8(token: &str, line: usize, field: &'static str) -> Result<u8, GoldenFixtureError> {
    token
        .parse()
        .map_err(|_| format_error_value(line, field, token))
}

fn parse_u16(token: &str, line: usize, field: &'static str) -> Result<u16, GoldenFixtureError> {
    token
        .parse()
        .map_err(|_| format_error_value(line, field, token))
}

fn parse_u32(token: &str, line: usize, field: &'static str) -> Result<u32, GoldenFixtureError> {
    token
        .parse()
        .map_err(|_| format_error_value(line, field, token))
}

fn parse_u64(token: &str, line: usize, field: &'static str) -> Result<u64, GoldenFixtureError> {
    token
        .parse()
        .map_err(|_| format_error_value(line, field, token))
}

fn parse_usize(token: &str, line: usize, field: &'static str) -> Result<usize, GoldenFixtureError> {
    token
        .parse()
        .map_err(|_| format_error_value(line, field, token))
}

fn parse_hex_u64(token: &str, line: usize, field: &'static str) -> Result<u64, GoldenFixtureError> {
    u64::from_str_radix(token, 16).map_err(|_| format_error_value(line, field, token))
}

fn parse_f32_bits(
    token: &str,
    line: usize,
    field: &'static str,
) -> Result<f32, GoldenFixtureError> {
    u32::from_str_radix(token, 16)
        .map(f32::from_bits)
        .map_err(|_| format_error_value(line, field, token))
}

fn parse_f32_vec(
    tokens: &[&str],
    line: usize,
    field: &'static str,
) -> Result<Vec<f32>, GoldenFixtureError> {
    tokens
        .iter()
        .map(|token| parse_f32_bits(token, line, field))
        .collect()
}

fn format_error<T>(line: usize, message: impl Into<String>) -> Result<T, GoldenFixtureError> {
    Err(GoldenFixtureError::Format {
        line,
        message: message.into(),
    })
}

fn format_error_value(line: usize, field: &'static str, token: &str) -> GoldenFixtureError {
    GoldenFixtureError::Format {
        line,
        message: format!("invalid {field}: {token}"),
    }
}

struct NumberedLines<'a> {
    lines: std::str::Lines<'a>,
    line_no: usize,
}

impl<'a> NumberedLines<'a> {
    fn new(contents: &'a str) -> Self {
        Self {
            lines: contents.lines(),
            line_no: 0,
        }
    }

    fn next_required(
        &mut self,
        context: &'static str,
    ) -> Result<(usize, &'a str), GoldenFixtureError> {
        self.next_nonempty()
            .ok_or_else(|| GoldenFixtureError::Format {
                line: self.line_no + 1,
                message: format!("expected {context}, reached end of file"),
            })
    }

    fn next_nonempty(&mut self) -> Option<(usize, &'a str)> {
        for line in self.lines.by_ref() {
            self.line_no += 1;
            if !line.trim().is_empty() {
                return Some((self.line_no, line));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::generate_fixture;

    #[test]
    fn golden_fixture_round_trips_exactly() {
        let config = FixtureConfig {
            agent_count: 8,
            task_count: 4,
            dimensions: 3,
            k: 2,
            seed: 0x5154_4f4d,
        };
        let fixture = generate_fixture(config);
        let encoded = encode_golden_fixture(config, &fixture).unwrap();
        let decoded = decode_golden_fixture(&encoded).unwrap();

        assert_eq!(decoded.config, config);
        assert_eq!(decoded.fixture, fixture);
    }

    #[test]
    fn golden_fixture_reports_bad_magic() {
        let error = decode_golden_fixture("wrong-format\n").unwrap_err();

        assert!(
            error
                .to_string()
                .contains("expected qtom-golden-fixture-v1")
        );
    }
}

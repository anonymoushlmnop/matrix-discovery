use chrono::{DateTime, Duration, Utc};
use dependency_types::{
    dependency::Dependency, existential::check_existential_dependency,
    temporal::check_temporal_dependency,
};
use std::collections::{HashMap, HashSet};

pub mod dependency_types;
pub mod parser;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Event {
    pub case: String,
    pub activity: char,
    pub predecessor: Option<String>,
}

#[derive(Debug)]
pub struct State {
    pub partition: Option<usize>,
    pub sequences: HashSet<Event>,
}

#[derive(Debug)]
pub struct ExtendedPrefixAutomaton {
    pub states: HashMap<String, State>,
    pub transitions: Vec<(String, char, String)>,
    pub activities: HashSet<char>,
    pub root: String,
}

impl Default for ExtendedPrefixAutomaton {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtendedPrefixAutomaton {
    pub fn new() -> Self {
        let root_id = "root".to_string();
        let mut states = HashMap::new();
        states.insert(
            root_id.clone(),
            State {
                partition: None,
                sequences: HashSet::new(),
            },
        );

        Self {
            states,
            transitions: Vec::new(),
            activities: HashSet::new(),
            root: root_id,
        }
    }

    pub fn build(plain_log: Vec<Vec<Event>>) -> Self {
        let mut epa = Self::new();
        let mut last_at: HashMap<String, String> = HashMap::new();

        for trace in plain_log {
            for event in trace {
                let pred_at = event
                    .predecessor
                    .as_ref()
                    .and_then(|case| last_at.get(case))
                    .unwrap_or(&epa.root)
                    .to_string();

                let current_at = if let Some(target) = epa
                    .transitions
                    .iter()
                    .find(|(source, act, _)| source == &pred_at && *act == event.activity)
                    .map(|(_, _, target)| target.to_string())
                {
                    target
                } else {
                    let new_state_id = format!("s{}", epa.states.len());
                    let current_c = if pred_at == epa.root {
                        1
                    } else if epa
                        .transitions
                        .iter()
                        .any(|(source, _, _)| source == &pred_at)
                    {
                        epa.states
                            .values()
                            .filter_map(|s| s.partition)
                            .max()
                            .unwrap_or(0)
                            + 1
                    } else {
                        epa.states[&pred_at].partition.unwrap_or(0)
                    };

                    epa.states.insert(
                        new_state_id.clone(),
                        State {
                            partition: Some(current_c),
                            sequences: HashSet::new(),
                        },
                    );

                    epa.transitions
                        .push((pred_at, event.activity, new_state_id.clone()));
                    epa.activities.insert(event.activity);

                    new_state_id
                };

                if let Some(state) = epa.states.get_mut(&current_at) {
                    state.sequences.insert(event.clone());
                }

                last_at.insert(event.case.clone(), current_at);
            }
        }

        epa
    }

    pub fn variant_entropy(&self) -> f64 {
        let s = (self.states.len() as f64).max(1.0);
        let s = if s > 1.0 { s - 1.0 } else { s };

        let partition_sizes: HashMap<usize, usize> = self
            .states
            .values()
            .filter_map(|state| state.partition)
            .fold(HashMap::new(), |mut acc, partition| {
                *acc.entry(partition).or_insert(0) += 1;
                acc
            });

        let sum_term: f64 = partition_sizes
            .values()
            .map(|&size| {
                let size_f64 = size as f64;
                size_f64 * size_f64.log10()
            })
            .sum();

        s * s.log10() - sum_term
    }

    pub fn normalized_variant_entropy(&self) -> f64 {
        let e_v = self.variant_entropy();
        let s = (self.states.len() as f64).max(1.0);
        let s = if s > 1.0 { s - 1.0 } else { s };

        if s * s.log10() == 0.0 {
            0.0 // Avoid division by zero
        } else {
            e_v / (s * s.log10())
        }
    }
}

pub fn generate_xes(text: &str) -> String {
    let mut output = String::with_capacity(text.len() * 2); // Estimate capacity

    // Parse the text to get traces and their frequencies
    let traces_with_frequencies = get_traces_with_frequencies(text);

    output.push_str("<log xes.version=\"1.0\" xes.features=\"nested-attributes\" openxes.version=\"1.0RC7\" xmlns=\"http://www.xes-standard.org/\">\n");

    for (trace, frequency) in traces_with_frequencies {
        // Repeat the trace based on the frequency
        for _ in 0..frequency {
            output.push_str("<trace>\n");

            let mut timestamp = DateTime::<Utc>::default();
            const EVENT_INTERVAL: i64 = 1000;

            for event in &trace {
                timestamp = timestamp
                    .checked_add_signed(Duration::milliseconds(EVENT_INTERVAL))
                    .expect("Time overflow occurred");

                output.push_str(&format!(
                    "<event>\n\
                    <string key=\"concept:name\" value=\"{}\"/>\n\
                    <date key=\"time:timestamp\" value=\"{}\"/>\n\
                    </event>\n",
                    event,
                    timestamp.to_rfc3339()
                ));
            }

            output.push_str("</trace>\n");
        }
    }

    output.push_str("</log>\n");

    output
}

/// Parse the text to get traces with their frequencies
fn get_traces_with_frequencies(text: &str) -> Vec<(Vec<&str>, usize)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            // Check if the line contains a frequency specified, and fallback to 1 if it doesn't
            let (trace_part, freq_part) = line.rsplit_once(':').unwrap_or((line, "1"));
            let frequency = freq_part.trim().parse::<usize>().unwrap_or(1);

            let trace: Vec<&str> = trace_part
                .split(',')
                .filter(|activity| !activity.trim().is_empty())
                .collect();

            if trace.is_empty() {
                None
            } else {
                Some((trace, frequency))
            }
        })
        .collect()
}

pub fn generate_adj_matrix_from_traces(
    traces: Vec<Vec<String>>,
    existential_threshold: f64,
    temporal_threshold: f64,
) -> (
    String,
    usize,
    usize,
    usize,
    usize,
    usize,
    HashMap<String, usize>,
) {
    let activities: HashSet<String> = traces
        .iter()
        .flat_map(|trace| trace.iter().cloned())
        .collect();

    generate_adj_matrix_from_activities_and_traces(
        &activities,
        traces,
        existential_threshold,
        temporal_threshold,
    )
}

pub fn generate_adj_matrix_from_activities_and_traces(
    activities: &HashSet<String>,
    traces: Vec<Vec<String>>,
    existential_threshold: f64,
    temporal_threshold: f64,
) -> (
    String,
    usize,
    usize,
    usize,
    usize,
    usize,
    HashMap<String, usize>,
) {
    const MAX_DEPENDENCY_WIDTH: usize = 15;

    let mut output =
        String::with_capacity(activities.len() * activities.len() * MAX_DEPENDENCY_WIDTH);
    let mut metrics = MatrixMetrics::default();

    // Header
    output.push_str(&format!("{:<MAX_DEPENDENCY_WIDTH$}", " "));

    let mut activities_sorted: Vec<_> = activities.iter().collect();
    activities_sorted.sort();

    for activity in &activities_sorted {
        output.push_str(&format!("{:<MAX_DEPENDENCY_WIDTH$}", activity));
    }
    output.push('\n');

    let format_dependency = |dep: &Dependency| {
        format!(
            "{:<width$}",
            format!("{}", dep),
            width = MAX_DEPENDENCY_WIDTH
        )
    };

    let converted_traces: Vec<Vec<&str>> = traces
        .iter()
        .map(|v| v.iter().map(|s| s.as_str()).collect())
        .collect();

    for from in &activities_sorted {
        output.push_str(&format!("{:<MAX_DEPENDENCY_WIDTH$}", from));

        for to in &activities_sorted {
            if to != from {
                let temporal_dependency =
                    check_temporal_dependency(from, to, &converted_traces, temporal_threshold);
                let existential_dependency = check_existential_dependency(
                    from,
                    to,
                    &converted_traces,
                    existential_threshold,
                );

                let dependency = Dependency::new(
                    from.to_string(),
                    to.to_string(),
                    temporal_dependency.clone(),
                    existential_dependency.clone(),
                );

                metrics.update(&temporal_dependency, &existential_dependency);

                output.push_str(&format_dependency(&dependency));
            } else {
                output.push_str(&format!("{:<MAX_DEPENDENCY_WIDTH$}", "TODO"));
            }
        }
        output.push('\n');
    }

    (
        output,
        metrics.full_independences,
        metrics.pure_existences,
        metrics.eventual_equivalences,
        metrics.direct_equivalences,
        activities.len(),
        metrics.relationship_counts,
    )
}

#[derive(Default)]
struct MatrixMetrics {
    full_independences: usize,
    pure_existences: usize,
    eventual_equivalences: usize,
    direct_equivalences: usize,
    relationship_counts: HashMap<String, usize>,
}

impl MatrixMetrics {
    fn update(
        &mut self,
        temporal_dependency: &Option<dependency_types::temporal::TemporalDependency>,
        existential_dependency: &Option<dependency_types::existential::ExistentialDependency>,
    ) {
        use dependency_types::{
            existential::DependencyType as EDType, temporal::DependencyType as TDType,
        };

        let temporal_type = match temporal_dependency {
            Some(td) => match td.dependency_type {
                TDType::Eventual => "eventual",
                TDType::Direct => "direct",
            },
            None => {
                self.pure_existences += 1;
                "none"
            }
        };

        let existential_type = match existential_dependency {
            Some(ed) => match ed.dependency_type {
                EDType::Equivalence => "equivalence",
                EDType::Implication => "implication",
                EDType::NegatedEquivalence => "negated equivalence",
                _ => "other",
            },
            None => {
                if temporal_type == "none" {
                    self.full_independences += 1;
                }
                "none"
            }
        };

        // Record relationship type
        let relationship_type = format!("({}, {})", temporal_type, existential_type);
        *self
            .relationship_counts
            .entry(relationship_type)
            .or_insert(0) += 1;

        // Check for equivalences
        if let Some(ed) = existential_dependency {
            if ed.dependency_type == EDType::Equivalence {
                if let Some(td) = temporal_dependency {
                    match td.dependency_type {
                        TDType::Eventual => self.eventual_equivalences += 1,
                        TDType::Direct => self.direct_equivalences += 1,
                    }
                }
            }
        }
    }
}

pub fn get_activities_and_traces(text: &str) -> (Vec<String>, Vec<Vec<&str>>) {
    let traces = get_traces(text);
    let activities: HashSet<String> = traces
        .iter()
        .flat_map(|trace| trace.iter().map(|&s| s.to_string()))
        .collect();

    (activities.into_iter().collect(), traces)
}

pub fn get_traces(text: &str) -> Vec<Vec<&str>> {
    text.lines()
        .filter_map(|line| {
            let trace: Vec<&str> = line
                .split(',')
                .filter(|&activity| !activity.trim().is_empty())
                .collect();

            if !trace.is_empty() {
                Some(trace)
            } else {
                None
            }
        })
        .collect()
}

// TODO: fix tests so they can be ran simultaneously and don't interfere with each other
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_activities_and_traces() {
        let traces = "
activity 3,activity 3,activity 3,activity 3,activity 3,activity 1,activity 1,activity 2,
activity 3,activity 1,activity 2,
activity 1,activity 1,activity 1,activity 1,activity 3,activity 1,activity 1,activity 2,
activity 3,activity 1,activity 1,activity 2,
";
        let (activities, traces) = get_activities_and_traces(traces);
        let expected_activities: HashSet<_> = vec!["activity 1", "activity 2", "activity 3"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(expected_activities, activities.into_iter().collect());

        let expected_traces = vec![
            vec![
                "activity 3",
                "activity 3",
                "activity 3",
                "activity 3",
                "activity 3",
                "activity 1",
                "activity 1",
                "activity 2",
            ],
            vec!["activity 3", "activity 1", "activity 2"],
            vec![
                "activity 1",
                "activity 1",
                "activity 1",
                "activity 1",
                "activity 3",
                "activity 1",
                "activity 1",
                "activity 2",
            ],
            vec!["activity 3", "activity 1", "activity 1", "activity 2"],
        ];
        assert_eq!(expected_traces, traces);
    }

    #[test]
    fn test_get_traces() {
        let traces = "
activity 3,activity 3,activity 3,activity 3,activity 3,activity 1,activity 1,activity 2,
activity 3,activity 1,activity 2,
activity 1,activity 1,activity 1,activity 1,activity 3,activity 1,activity 1,activity 2,
activity 3,activity 1,activity 1,activity 2,
";
        let expected_traces = vec![
            vec![
                "activity 3",
                "activity 3",
                "activity 3",
                "activity 3",
                "activity 3",
                "activity 1",
                "activity 1",
                "activity 2",
            ],
            vec!["activity 3", "activity 1", "activity 2"],
            vec![
                "activity 1",
                "activity 1",
                "activity 1",
                "activity 1",
                "activity 3",
                "activity 1",
                "activity 1",
                "activity 2",
            ],
            vec!["activity 3", "activity 1", "activity 1", "activity 2"],
        ];
        assert_eq!(expected_traces, get_traces(traces));
    }
}

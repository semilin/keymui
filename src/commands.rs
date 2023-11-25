use crate::{Keymui, Message};
use iced::Application;

#[derive(Debug, Clone)]
pub enum UserArg {
    Key,
    NaturalNum,
    String,
}

#[derive(Debug, Clone, Copy)]
pub enum UserCommand {
    SetMetricsDirectory,
    ReloadMetrics,
    ImportCorpus,
    ViewNotification,
    Swap,
    Precision,
    NgramFrequency,
}

impl UserCommand {
    pub fn args(self) -> Vec<UserArg> {
        match self {
            UserCommand::SetMetricsDirectory => vec![],
            UserCommand::ReloadMetrics => vec![],
            UserCommand::ImportCorpus => vec![],
            UserCommand::ViewNotification => vec![],
            UserCommand::Swap => vec![UserArg::Key, UserArg::Key],
            UserCommand::Precision => vec![UserArg::NaturalNum],
            UserCommand::NgramFrequency => vec![UserArg::String, UserArg::String],
        }
    }
}

impl std::fmt::Display for UserCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserCommand::SetMetricsDirectory => write!(f, "set-metrics-directory"),
            UserCommand::ReloadMetrics => write!(f, "reload-metrics"),
            UserCommand::ImportCorpus => write!(f, "import-corpus"),
            UserCommand::ViewNotification => write!(f, "view-notification"),
            UserCommand::Swap => write!(f, "swap"),
            UserCommand::Precision => write!(f, "precision"),
            UserCommand::NgramFrequency => write!(f, "ngram-frequency"),
        }
    }
}

pub fn linear_matches(src: &str, target: &str) -> bool {
    if target.len() < src.len() {
        return false;
    }
    for (a, b) in src.chars().zip(target.chars()) {
        if a != b {
            return false;
        }
    }
    true
}

pub fn match_list(src: &str, choices: &Vec<String>) -> Vec<usize> {
    choices
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if linear_matches(src, c) {
                Some(i)
            } else {
                None
            }
        })
        .collect()
}

pub fn commonest_completion(matches: &Vec<&str>) -> usize {
    if matches.len() == 0 {
        return 0;
    } else if matches.len() == 1 {
        return matches[0].len();
    }

    for i in 0..matches[0].len() {
        let first = matches[0].chars().nth(i);
        if !matches.iter().all(|&x| x.chars().nth(i) == first) {
            return i;
        }
    }

    return 0;
}

impl Keymui {
    pub fn parse_command(&mut self) {
        let input = self.command_input.clone();
        let split: Vec<&str> = input.split_whitespace().collect();
        if split.len() == 0 {
            return;
        }
        let command = self
            .commands
            .iter()
            .map(|x| *x)
            .find(|c| c.to_string() == split[0]);
        if let Some(cmd) = command {
            let args: Vec<&str> = split
                .iter()
                .skip(1)
                .take(cmd.args().len())
                .map(|x| *x)
                .collect();

            self.run_command(&cmd, &args);

            self.command_input = String::new();
            self.filter_commands();
        }
    }

    pub fn run_command(&mut self, cmd: &UserCommand, args: &[&str]) {
        let message = match cmd {
            UserCommand::SetMetricsDirectory => Some(Message::SetMetricsDirectory),
            UserCommand::ReloadMetrics => Some(Message::ReloadMetrics),
            UserCommand::ImportCorpus => Some(Message::ImportNewCorpus),
            UserCommand::ViewNotification => Some(Message::ViewNotification),
            UserCommand::Swap => {
                let keys: Vec<char> = args.iter().filter_map(|x| x.chars().next()).collect();
                println!("{:?}", keys);
                if keys.len() == 2 {
                    Some(Message::SwapKeys(keys[0], keys[1]))
                } else {
                    None
                }
            }
            UserCommand::Precision => {
                if let Some(arg) = args.iter().next() {
                    let num: Result<u32, _> = arg.parse();
                    if let Ok(num) = num {
                        Some(Message::SetPrecision(num))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            UserCommand::NgramFrequency => {
                if let Some(ctx) = &self.metric_context {
                    let corpus = &ctx.analyzer.corpus;
                    let corpus_total = corpus.chars.iter().sum::<u32>() as f32;
                    let total: Vec<f32> = args
                        .iter()
                        .map(|arg| {
                            let chars: Vec<usize> = arg
                                .chars()
                                .map(|c| corpus.corpus_char(c))
                                .cloned()
                                .collect();
                            let freqs: [u32; 2] = match &chars[..] {
                                [a] => [corpus.chars[*a], 0],
                                [a, b] => {
                                    let idx = corpus.bigram_idx(*a, *b);
                                    [corpus.bigrams[idx], corpus.skipgrams[idx]]
                                }
                                [a, b, c] => {
                                    let idx = corpus.trigram_idx(*a, *b, *c);
                                    [corpus.trigrams[idx], 0]
                                }
                                _ => [0, 0],
                            };
                            freqs
                        })
                        .fold([0, 0], |[t1, t2], [n1, n2]| [t1 + n1, t2 + n2])
                        .iter()
                        .map(|x| 100.0 * *x as f32 / corpus_total)
                        .collect();
                    self.notification =
                        (format!("total: ({:.2}%, {:.2}%)", total[0], total[1]), None);
                }
                None
            }
        };
        if let Some(m) = message {
            let _ = self.update(m);
        }
    }

    pub fn filter_commands(&mut self) {
        self.input_completions = match_list(&self.command_input, &self.input_options);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub name: [u8; 2],
    pub data: Vec<u8>,
    pub offset: usize,
}

impl Command {
    pub fn mnemonic(&self) -> String {
        String::from_utf8_lossy(&self.name).into_owned()
    }

    pub fn numbers(&self) -> Vec<f64> {
        parse_numbers(&self.data)
    }
}

pub fn tokenize(input: &[u8]) -> Vec<Command> {
    let mut commands = Vec::new();
    let mut index = 0;
    let mut label_terminator = 3_u8;

    while index < input.len() {
        if input[index] == 0x1b {
            index = skip_escape(input, index);
            continue;
        }
        if input[index].is_ascii_whitespace() || input[index] == b';' {
            index += 1;
            continue;
        }
        if index + 1 >= input.len()
            || !input[index].is_ascii_uppercase()
            || !input[index + 1].is_ascii_uppercase()
        {
            index += 1;
            continue;
        }

        let offset = index;
        let name = [input[index], input[index + 1]];
        index += 2;
        let start = index;

        if &name == b"LB" {
            while index < input.len() && input[index] != label_terminator {
                index += 1;
            }
            let data = input[start..index].to_vec();
            if index < input.len() {
                index += 1;
            }
            commands.push(Command { name, data, offset });
            continue;
        }

        if &name == b"PE" {
            while index < input.len() && input[index] != b';' {
                index += 1;
            }
        } else {
            while index < input.len() {
                if input[index] == b';' {
                    break;
                }
                if input[index] == 0x1b {
                    break;
                }
                if index + 1 < input.len()
                    && input[index].is_ascii_uppercase()
                    && input[index + 1].is_ascii_uppercase()
                {
                    break;
                }
                index += 1;
            }
        }

        let data = input[start..index].to_vec();
        if &name == b"DT" && !data.is_empty() {
            label_terminator = data[0];
        }
        commands.push(Command { name, data, offset });
        if index < input.len() && input[index] == b';' {
            index += 1;
        }
    }
    commands
}

fn skip_escape(input: &[u8], mut index: usize) -> usize {
    index += 1;
    if input.get(index) == Some(&b'%') {
        index += 1;
        while index < input.len() {
            let byte = input[index];
            index += 1;
            if byte.is_ascii_alphabetic() {
                break;
            }
        }
        return index;
    }

    while index < input.len() {
        let byte = input[index];
        index += 1;
        if (0x40..=0x7e).contains(&byte) {
            break;
        }
    }
    index
}

pub fn parse_numbers(data: &[u8]) -> Vec<f64> {
    let text = String::from_utf8_lossy(data);
    text.split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<f64>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_classic_and_concatenated_commands() {
        let commands = tokenize(b"IN;PUPA10,20;PDPA30,40;SP0;");
        let names: Vec<_> = commands.iter().map(Command::mnemonic).collect();
        assert_eq!(names, ["IN", "PU", "PA", "PD", "PA", "SP"]);
        assert_eq!(commands[2].numbers(), [10.0, 20.0]);
    }

    #[test]
    fn skips_hpgl2_escape_and_reads_unterminated_commands() {
        let commands = tokenize(b"\x1b%-1BBPINPS96016,49097SP1QL0\r\nPU10,20PD 30,40PG;");
        let names: Vec<_> = commands.iter().map(Command::mnemonic).collect();
        assert_eq!(names, ["BP", "IN", "PS", "SP", "QL", "PU", "PD", "PG"]);
    }

    #[test]
    fn label_consumes_embedded_uppercase_pairs() {
        let commands = tokenize(b"DT@;PU1,2;LBHELLO HPGL@PD3,4;");
        assert_eq!(commands[2].mnemonic(), "LB");
        assert_eq!(String::from_utf8_lossy(&commands[2].data), "HELLO HPGL");
        assert_eq!(commands[3].mnemonic(), "PD");
    }
}

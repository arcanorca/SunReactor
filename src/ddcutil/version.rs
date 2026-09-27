#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdcutilVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl DdcutilVersion {
    pub fn parse(output: &str) -> Option<Self> {
        for line in output.lines() {
            let line = line.trim();
            if line.starts_with("ddcutil") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                for part in parts {
                    let segments: Vec<&str> = part.split('.').collect();
                    if segments.len() >= 3 {
                        if let (Ok(major), Ok(minor), Ok(patch)) = (
                            segments[0].parse::<u32>(),
                            segments[1].parse::<u32>(),
                            segments[2].parse::<u32>(),
                        ) {
                            return Some(Self {
                                major,
                                minor,
                                patch,
                            });
                        }
                    }
                }
            }
        }
        None
    }
}

//! Configuration for the Hoon formatter.

/// Configuration options for the formatter.
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Maximum line width (default: 80).
    pub max_width: usize,
    /// Preferred line width for breaking decisions (default: 56).
    pub preferred_width: usize,
    /// Indentation width in spaces (default: 2).
    pub indent_width: usize,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            max_width: 80,
            preferred_width: 56,
            indent_width: 2,
        }
    }
}

impl FormatterConfig {
    /// Create a new configuration with the given max width.
    pub fn with_max_width(mut self, width: usize) -> Self {
        self.max_width = width;
        self
    }

    /// Create a new configuration with the given preferred width.
    pub fn with_preferred_width(mut self, width: usize) -> Self {
        self.preferred_width = width;
        self
    }

    /// Create a new configuration with the given indent width.
    pub fn with_indent_width(mut self, width: usize) -> Self {
        self.indent_width = width;
        self
    }
}

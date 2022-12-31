use std::io::{BufReader, BufRead};
use std::option::Option;
use std::result::Result;
use std::fs::File;
use std::vec::Vec;
use walkdir::DirEntry;
use tokei::{Config, Languages, LanguageType};
use snafu::prelude::*;


/// This error is returned if a file is unabled to be parsed due to an
/// unknown extension. It should never get to this point as there is
/// layered parsing, but just in case
#[derive(Debug, Snafu)]
pub enum FileParserError
{
    #[snafu(display("The file '{file}' has a bad extension and could not be parsed"))]
    BadFileExtension { file: String },
}

/// Struct representing a valid file to be parsed
pub struct FileParser<'a>
{
    /// The name of the file being parsed, without the directories
    pub filename: String,
    /// Raw DirEntry type
    entry: &'a DirEntry,
    /// Mean function cyclomatic complexity for the file. Used for the Treemap.
    pub cc: Option<f64>,
    /// Number of lines of code for the file. Used for the Treemap.
    pub nloc: Option<u64>,
    /// The parent directory that the file is in. Used for the Treemap.
    pub parent: Option<String>,
    /// The path to the file from the root, including flename. Used for the
    /// Treemap
    pub label: Option<String>
}

/// Check if the file extension can be parsed by this program. Return TRUE if
/// it can, FALSE if it cannot.
/// Currently supported extensions are for C, C++, Python, and Javascript
pub fn is_file_extension_valid(file: &str) -> bool
{
    let extensions = vec![".c", ".cpp", ".cc", ".cxx", ".py", ".js"];

    extensions.iter()
              .any(|n| file.ends_with(*n))
}

/// Check if a directory is hidden. Return TRUE if hidden, FALSE if not
pub fn is_hidden(entry: &DirEntry) -> bool
{
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}


impl<'a> FileParser<'_>
{
    pub fn new (entry: &'a DirEntry) -> FileParser<'a>
    {
        FileParser
        {
            filename: entry.file_name().to_os_string().into_string().unwrap(),
            entry: entry,
            cc: None,
            nloc: None,
            parent: None,
            label: None
        }
    }

    /// Walk through a file, retrieving the cumulative complexity and the number
    /// of lines of code. Also parses the file path to extract the values for the
    /// Treemap, returning successfully if this is successful and returning
    /// an error if the file is otherwise unable to be parsed
    pub fn file_walk(&mut self) -> Result<(), FileParserError>
    {
        /* first get the mean of function complexities for the file */
        match self.get_file_complexity()
        {
            Some(complexity) => self.cc = Some(complexity),
            _ => {
                return BadFileExtensionSnafu
                {
                    file: &self.filename,
                }.fail()
            }
        }

        /* then get the nloc for the file */
        match self.get_file_nloc()
        {
            Some(nloc) => self.nloc = Some(nloc),
            _ => {
                return BadFileExtensionSnafu
                {
                    file: &self.filename,
                }.fail()
            }
        }

        /* finally set the values as vec elements for the treemap */
        let depth = self.entry.depth();

        let len = self.entry.path().to_str().unwrap()
                                   .split("/").count();

        let mut full_path = self.entry.path().to_str().unwrap()
                                  .split("/")
                                  .collect::<Vec<&str>>();

        /* the label is /path/to/file.c */
        self.label = Some(full_path[len-depth-1..].join("/"));

        full_path.pop();

        /* the parent is /path/to */
        self.parent = Some(full_path[len-depth-1..].join("/"));
        Ok(())
    }

    /// Get the file extension given a file name
    fn get_file_extension(&mut self) -> &str
    {
        /* fragile to multiple extensions but that is such an unlikely edge case */
        match self.filename.as_str().rsplit(".").next().unwrap()
        {
            "c" => "c",
            "cc" => "cpp",
            "cxx" => "cpp",
            "cpp" => "cpp",
            "py" => "py",
            "js" => "js",
            _ => ""
        }
    }

    /// Get the mean function complexity in a file by manually searching for
    /// decision statements and logical operations
    /// NOTE: Accuracy is questionable but the estimated complexity _should_
    /// be close to the actual. HOWEVER its magitudes better than the
    /// previous method of generating ASTs since there is a dearth of libraries
    /// for rust that can generate accurate ASTs for other languages.
    /// tree-sitter is awesome but was very fragile when dealing with
    /// C/C++ preprocessor directives. doing it the below way is simpler and
    /// returns a reasonable approximation of the actual cyclomatic complexity.
    fn get_file_complexity(&mut self) -> Option<f64>
    {
        let mut comments: Vec<&str> = Vec::new();
        let mut statements: Vec<&str> = Vec::new();
        let mut logical_ops: Vec<&str> = Vec::new();
        let function_def: &str;

        /* identify the extension */
        match self.get_file_extension()
        {
            "c" => {
                comments.extend(["//", "/*", "*/", "*", "///"].iter());
                statements.extend(["if(", "if (", "for(", "for (", "while(", "while (", "switch", "break", "goto"].iter());
                logical_ops.extend(["&&", "||"].iter());
                function_def = "return";

            },
            "cpp" => {
                comments.extend(["//", "/*", "*/", "*", "///"].iter());
                statements.extend(["if(", "if (", "for(", "for (", "while(", "while (", "switch", "break", "goto"].iter());
                logical_ops.extend(["&&", "||"].iter());
                function_def = "return";
            },
            "py" => {
                /* TODO */
                comments.extend(["#"].iter());
                statements.extend(["if", "for", "while", "break"].iter());
                logical_ops.extend(["and", "or", "not"].iter());
                function_def = "def ";
            },
            "js" => {
                /* TODO */
                comments.extend(["//", "*/", "/*"].iter());
                statements.extend(["if", "for", "while"].iter());
                logical_ops.extend(["&&", "||"].iter());
                function_def = "function";
            },
            _ => { return None; },
        }

        let mut logical_ops_count: u64 = 0;
        let mut function_count: u64 = 0;

        let path = self.entry.path();
        let f = File::open(&path).unwrap();
        let reader = BufReader::new(f).lines();

        /* this is how the iterator works:
         * - nukes any comment lines because it might fuck with the keyword searching
         * - check for logical operations, which may occur on a line more than once
         * - check for a function definition (this is very guess-y). for C/C++ it counts
         * the number of returns. some functions may have more than one, and some functions
         * may have none. hopefully it evens out.
         * - search for keywords (language specific) and nuke lines that don't have em
         * - collect it all into a vec. the size is the number of keywords
         * - add to this the number of logical operations counted
         * - done */

        let valid_lines: Vec<String> = reader.map(|x| x.unwrap())
                                    .filter(|x| comments.iter().all(|n| !x.contains(*n)))
                                    .inspect(|x| {
                                        /* estimating number of logical operations */
                                        for item in &logical_ops
                                        {
                                            logical_ops_count += if x.contains(item) { 1 } else { 0 };
                                        }

                                        /* estimating number of functions */
                                        function_count += if x.contains(function_def) { 1 } else { 0 };
                                        })
                                    .filter(|s| statements.iter().any(|n| s.contains(*n)))
                                    .collect();

        let mut complexity_count: u64 = valid_lines.len().try_into().unwrap();
        complexity_count += logical_ops_count;

        let mean_complexity: f64;

        if function_count == 0
        {
            mean_complexity = 0.0;
        }
        else
        {
            mean_complexity = complexity_count as f64 / function_count as f64;
        }

        return Some(mean_complexity);
    }

    /// Get the number of lines of code in a file
    fn get_file_nloc(&mut self) -> Option<u64>
    {
        let path = &[self.entry.path().to_str().unwrap()];
        let excluded = &[];

        let config = Config::default();
        let mut languages = Languages::new();

        languages.get_statistics(path, excluded, &config);

        /* manually identify the extension */
        match self.get_file_extension()
        {
            "c" => {
                let lang = &languages[&LanguageType::C];
                Some(lang.code.try_into().unwrap())
            },
            "cpp" => {
                let lang = &languages[&LanguageType::Cpp];
                Some(lang.code.try_into().unwrap())
            },
            "py" => {
                let lang = &languages[&LanguageType::Python];
                Some(lang.code.try_into().unwrap())
            },
            "js" => {
                let lang = &languages[&LanguageType::JavaScript];
                Some(lang.code.try_into().unwrap())
            },
            _ => None,
        }
    }
}

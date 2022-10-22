use std::io::{BufReader, Read};
use std::option::Option;
use std::result::Result;
use std::fs::File;
use std::vec::Vec;
use walkdir::DirEntry;
use tree_sitter::{Node, Tree};
use tree_sitter::Parser as TreeParser;
use tokei::{Config, Languages, LanguageType};
use snafu::prelude::*;


/// This error is returned if a file is unabled to be parsed due to an
/// unknown extension. It should never get to this point as there is
/// layered parsing, but just in case
#[derive(Debug, Snafu)]
pub enum FileParserError{
    #[snafu(display("The file '{file}' has a bad extension and could not be parsed"))]
    BadFileExtension { file: String },
}

/// Struct representing a valid file to be parsed
pub struct FileParser<'a> {
    /// The name of the file being parsed, without the directories
    pub filename: String, 
    /// Raw DirEntry type
    entry: &'a DirEntry,
    /// Cyclomatic complexity for the file. Used for the Treemap.
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
/// Currently supported extensions are: .c, .cpp, .cc, and .cxx
pub fn is_file_extension_valid(file: &str) -> bool {
    let extensions = vec![".c", ".cpp", ".cc", ".cxx"];

    extensions.iter()
              .any(|n| file.ends_with(*n))
}

/// Check if a directory is hidden. Return TRUE if hidden, FALSE if not
pub fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name()
         .to_str()
         .map(|s| s.starts_with("."))
         .unwrap_or(false)
}


impl<'a> FileParser<'_> {
    pub fn new (entry: &'a DirEntry) -> FileParser<'a> {
        FileParser {
            filename: entry.file_name().to_os_string().into_string().unwrap(),
            entry: entry,
            cc: None,
            nloc: None,
            parent: None,
            label: None
        }
    }

    /// Parse a compound statement in a function. The compound statement is the
    /// body of an if/for/while/etc. statement. Return the computed cyclomatic
    /// complexity of the statement
    fn parse_compound_statement(&mut self, root_node: &Node, tree: &Tree) -> u64 {

        /* this is the complexity for the file */
        let mut complexity: u64 = 0;

        /* decision statements taken from
         * http://sarnold.github.io/cccc/CCCC_User_Guide.html */
        let decision_statements = vec!["if_statement",
                                       "for_statement",
                                       "while_statement",
                                       "switch_statement",
                                       "break_statement",
                                       "goto_statement"];

        for node in root_node.children(&mut tree.walk()) {
            /* if the node is one of the decision statements */
            if decision_statements.iter().any(|n| *n == node.kind()) {
                complexity += 1;
            }

            for node2 in node.children(&mut tree.walk()) {
                /* if there is a nested decision statement */
                if node2.kind() == "compound_statement" {
                    complexity += self.parse_compound_statement(&node2, &tree);
                }

                /* checking for && or || since these introduce additional
                 * paths */
                /* TODO yikes */
                if node2.kind() == "parenthesized_expression" {
                    for node3 in node2.children(&mut tree.walk()) {
                        if node3.kind() == "binary_expression" {
                            for node4 in node3.children(&mut tree.walk()) {
                                if node4.kind() == "&&" || node4.kind() == "||" {
                                    complexity += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        complexity
    }

    /// Get the file extension given a file name
    fn get_file_extension(&mut self) -> &str {

        if self.filename.ends_with(".c") {
            "c"
        }
        else if is_file_extension_valid(&self.filename) {
            /* hacky way to do it that will only work if the extensions are C
             * (covered in the if above) and C++ */
            "cpp"
        }
        else
        {
            ""
        }
    }

    /// Get the cumulative complexity in a file by parsing all the
    /// compound statements in decision statements, including nested
    /// decision statements
    fn get_file_complexity(&mut self) -> Option<f64> {

        let path = self.entry.path();

        /* vec to store the complexity of each function */
        let mut func_complexities = Vec::new();

        /* open the file */
        let f = File::open(&path).unwrap();
        let mut reader = BufReader::new(f);
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer).unwrap();

        let mut parser = TreeParser::new();

        /* manually identify the extension */
        match self.get_file_extension() {
            "c" => {
                parser.set_language(tree_sitter_c::language())
                      .expect("Error loading C grammar");
            },
            "cpp" => {
                parser.set_language(tree_sitter_cpp::language())
                      .expect("Error loading C++ grammar");
            },
            _ => { return None; },
        }

        let tree = parser.parse(buffer, None).unwrap();

        /* explores the nodes in the AST. to get an idea of what the AST
         * looks like, see
         * https://github.com/tree-sitter/tree-sitter-c/blob/master/test/corpus/expressions.txt
         */
        for node in tree.root_node().children(&mut tree.walk()) {
            /* parse a function */
            if node.kind() == "function_definition" {

                let mut complexity = 0;

                for node2 in node.children(&mut tree.walk()) {
                    /* go through the nodes in the function */
                    if node2.kind() == "compound_statement" {
                        /* computes the complexity of a compound or nested compound
                         * statement */
                        complexity += self.parse_compound_statement(&node2, &tree); 
                    }
                }

                /* push the complexity of the function */
                func_complexities.push(complexity);
            }
        }

        /* compute avg in the file */
        let sum = func_complexities.iter().sum::<u64>() as f64;

//        let count = func_complexities.len();
//        let mean: f64;
//
//        /* TODO: hacky */
//        if sum == 0.0 {
//            mean = 0.0;
//            eprintln!("mean function complexity for {:?} set to 0. likely bad AST parse", &self.filename);
//        } else {
//            mean = sum / count as f64;
//        }

        return Some(sum);
    }

    /// Get the number of lines of code in a file
    fn get_file_nloc(&mut self) -> Option<u64> {

        let path = &[self.entry.path().to_str().unwrap()];
        let excluded = &[];

        let config = Config::default();
        let mut languages = Languages::new();

        languages.get_statistics(path, excluded, &config);

        /* manually identify the extension */
        match self.get_file_extension() {
            "c" => {
                let lang = &languages[&LanguageType::C];
                Some(lang.code.try_into().unwrap())
            },
            "cpp" => {
                let lang = &languages[&LanguageType::Cpp];
                Some(lang.code.try_into().unwrap())
            },
            _ => None,
        }
    }

    /// Walk through a file, retrieving the cumulative complexity and the number
    /// of lines of code. Also parses the file path to extract the values for the
    /// Treemap, returning successfully if this is successful and returning
    /// an error if the file is otherwise unable to be parsed
    pub fn file_walk(&mut self) -> Result<(), FileParserError> {

        /* first get the mean of function complexities for the file */
        match self.get_file_complexity() {
            Some(complexity) => self.cc = Some(complexity),
            _ => {
                return BadFileExtensionSnafu {
                    file: &self.filename,
                }.fail()
            }
        }

        /* then get the nloc for the file */
        match self.get_file_nloc() {
            Some(nloc) => self.nloc = Some(nloc),
            _ => {
                return BadFileExtensionSnafu {
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
}

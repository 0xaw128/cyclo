use std::{fs,assert_eq,assert};
use std::io::Write;
use std::path::PathBuf;
use std::vec::Vec;
use clap::Parser;
use walkdir::WalkDir;

mod file_parser;

use file_parser::FileParser;


#[derive(Parser,Debug)]
#[clap(name="cyclo", about="visualize complexity")]
struct Args
{
    /// Relative path to directory to analyze
    #[clap(short = 'p', long, value_parser)]
    path: PathBuf,
    /// Whether to write a debug file
    #[clap(short = 'd', long, action)]
    debug: bool,
}

fn main()
{
    let args = Args::parse();

    let walker = WalkDir::new(&args.path).into_iter();

    let mut nlocs = Vec::new();
    let mut labels = Vec::new();
    let mut parents = Vec::new();
    let mut ccs = Vec::new();

    /* TODO: multithreading. performance isn't a massive issue atm though */
    /* parse each file and calculate complexity */
    for entry in walker.filter_entry(|e| !file_parser::is_hidden(e))
    {
        if file_parser::is_file_extension_valid(&entry.as_ref().unwrap()
                                                      .file_name()
                                                      .to_str().unwrap())
        {
            let mut file = FileParser::new(&entry.as_ref().unwrap());

            match file.file_walk()
            {
                Ok(()) => {
                    nlocs.push(file.nloc.unwrap());
                    ccs.push(file.cc.unwrap());
                    labels.push(file.label.unwrap().clone());
                    parents.push(file.parent.unwrap().clone());
                },
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    continue;
                }
            }

            /* dumb to do this again but it works */
            let depth = entry.as_ref().unwrap().depth();
            let len = entry.as_ref().unwrap().path().to_str().unwrap()
                           .split("/").count();

            let mut full_path = entry.as_ref().unwrap().path().to_str().unwrap()
                                     .split("/")
                                     .collect::<Vec<&str>>();

            /* pop to remove filename from path */
            full_path.pop();

            /* loop through and check if the parent dirs are in the parent and label vecs */
            for _ in 0..depth
            {
                /* check if the path is a parent */

                /* if the parent path does not exist in the parent vec */
                if !labels.contains(&full_path[len-depth-1..].join("/"))
                {
                    nlocs.push(0);
                    ccs.push(0.0);
                    labels.push(full_path[len-depth-1..].join("/"));

                    full_path.pop();

                    if full_path.is_empty()
                    {
                        parents.push("".to_string());

                    }
                    else
                    {
                        parents.push(full_path[len-depth-1..].join("/"));
                    }
                }
            }
        }
    }

    /* test lengths of the vecs, since they must all be the same */
    assert_eq!(nlocs.len(), labels.len(), "nloc ({}) and label ({}) vector length equality failed", nlocs.len(), labels.len());
    assert_eq!(labels.len(), parents.len(), "labels ({}) and parents ({}) vector length equality failed", labels.len(), parents.len());
    assert_eq!(parents.len(), ccs.len(), "parents ({}) and ccs ({}) vector lengthe equality failed", parents.len(), ccs.len());


    /* write the js file */
    {
        let sum = ccs.iter().sum::<f64>();
        let count = ccs.len();

        assert!(count > 0, "count ({}) is not greater than zero", count);

        let mean = sum / count as f64;

        let js_file = format!(r#"
var jsondata = [{{
        type: "treemap",
        values: {:?},
        labels: {:?},
        parents: {:?},
        marker: {{colors: {:.2?}, cmid: {:.2?}, colorscale: "Greens"}}
}}]
    "#, nlocs, labels, parents, ccs, mean);

        fs::write("../webserver/web/scripts/cyclo.js", js_file).unwrap();
    }


    if args.debug
    {
        /* write the debug file */
        let mut buffer = fs::File::create("debug.txt").unwrap();

        for i in 0..nlocs.len()
        {
            writeln!(&mut buffer, "file: {:?}, nloc: {:?}, cc: {:?}", labels[i], nlocs[i], ccs[i]).unwrap();
        }
    }
}

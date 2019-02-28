use std::env;
use std::fs;
use std::io;

use std::collections::BTreeSet;
use std::io::Read;
use std::iter::FromIterator;

use itertools::Itertools;
use rand::seq::SliceRandom;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub struct Photo {
    idx: usize,
    orient: Orientation,
    nb_tags: usize,
    tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub enum Slide {
    Vertical(Photo, Photo),
    Horizontal(Photo),
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Serialize, Deserialize)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

fn deserialize_slide(slide: Slide) -> String {
    match slide {
        Slide::Vertical(Photo { idx: idx1, .. }, Photo { idx: idx2, .. }) => {
            format!("{0} {1}", idx1, idx2)
        }
        Slide::Horizontal(Photo { idx, .. }) => format!("{0}", idx),
    }
}

fn deserialize(slideshow: Vec<Slide>) -> String {
    let len = slideshow.len();
    let output = slideshow.into_iter().map(deserialize_slide).join("\n");
    format!("{0}\n{1}", len, output)
}

fn score_slide(s1: &Slide, s2: &Slide) -> u32 {
    let (mut tags, uniques) = match s1 {
        Slide::Vertical(Photo { tags: t1, .. }, Photo { tags: t2, .. }) => (
            t1.iter().chain(t2.iter()).collect::<Vec<_>>(),
            BTreeSet::from_iter(t1.iter().chain(t2.iter())),
        ),
        Slide::Horizontal(Photo { tags, .. }) => (
            tags.iter().collect::<Vec<_>>(),
            BTreeSet::from_iter(tags.iter()),
        ),
    };

    let (mut tags2, uniques2) = match s2 {
        Slide::Vertical(Photo { tags: t1, .. }, Photo { tags: t2, .. }) => (
            t1.iter().chain(t2.iter()).collect::<Vec<_>>(),
            BTreeSet::from_iter(t1.iter().chain(t2.iter())),
        ),
        Slide::Horizontal(Photo { tags, .. }) => (
            tags.iter().collect::<Vec<_>>(),
            BTreeSet::from_iter(tags.iter()),
        ),
    };

    let (all_tags, all_uniques) = {
        let mut all_tags = Vec::with_capacity(tags.len() + tags2.len());
        all_tags.append(&mut tags);
        all_tags.append(&mut tags2);
        (all_tags, uniques.union(&uniques2).collect::<BTreeSet<_>>())
    };

    100 - (all_uniques.len() as u32 / all_tags.len() as u32) * 100
}

fn score_slideshow(s1: &[Slide]) -> u32 {
    s1.iter()
        .zip(s1.iter().skip(1))
        .map(|(s1, s2)| score_slide(s1, s2))
        .sum()
}

fn generate_slideshow(photos: Vec<Photo>) -> Option<Vec<Slide>> {
    let (mut verticals, horizontals): (Vec<Photo>, Vec<Photo>) =
        photos
            .into_iter()
            .partition(|Photo { orient, .. }| match orient {
                Orientation::Vertical => true,
                Orientation::Horizontal => false,
            });

    let mut record: Option<Vec<Slide>> = None;
    let mut rng = rand::thread_rng();

    for i in 0..100 {
        println!("Iteration {} / {}", i, 100);
        verticals.as_mut_slice().shuffle(&mut rng);
        let vslides = verticals
            .chunks(2)
            // .combinations(2)
            .map(|v| Slide::Vertical(v[0].clone(), v[1].clone()));

        let all_slides: Vec<Slide> = horizontals
            .clone()
            .into_iter()
            .map(Slide::Horizontal)
            .chain(vslides)
            .collect::<Vec<Slide>>();

        let len = all_slides.len();
        let all_combs = all_slides.into_iter().combinations(len);

        let max = all_combs
            .max_by(|s1, s2| score_slideshow(s1.as_slice()).cmp(&score_slideshow(s2.as_slice())));

        if let Some(max) = max {
            if let Some(ref r) = record {
                if score_slideshow(max.as_slice()) > score_slideshow(r.as_slice()) {
                    record = Some(max.to_vec());
                }
            } else {
                record = Some(max.to_vec());
            }
        }
    }

    record
}

fn main() -> io::Result<()> {
    let path = match env::args().nth(1) {
        Some(arg) => arg,
        None => {
            eprintln!("Missing argument");
            return Ok(());
        }
    };
    let rg = Regex::new(r#"^(H|V)\s+(\d+)\s+(\w+(?:\s+\w+)*)$"#).unwrap();

    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let mut iter = contents.split('\n');
    let first = iter.next().and_then(|x| x.parse().ok()).unwrap();
    let mut photos = Vec::with_capacity(first);

    for (idx, line) in iter.filter(|line| !str::is_empty(line)).enumerate() {
        let captures = match rg.captures(line) {
            Some(captures) => captures,
            None => {
                eprintln!("Capture failed for: {0}", line);
                return Ok(());
            }
        };

        let orient = match &captures[1] {
            "H" => Orientation::Horizontal,
            "V" => Orientation::Vertical,
            _ => unreachable!(),
        };
        let nb_tags = captures[2].parse().expect("Invalid number of tags");
        let tags = captures[3]
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();
        if nb_tags != tags.len() {
            eprintln!("Wrong number of tags.");
            return Ok(());
        }

        photos.push(Photo {
            idx,
            orient,
            nb_tags,
            tags,
        });
    }

    fs::write(
        "out.txt",
        generate_slideshow(photos)
            .map(deserialize)
            .unwrap_or_else(|| "0\n".to_string()),
    )?;

    Ok(())
}

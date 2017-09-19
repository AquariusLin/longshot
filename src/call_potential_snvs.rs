extern crate rust_htslib;

use rust_htslib::bam;
use rust_htslib::prelude::*;
use bio::io::fasta;
use std::char;
use util::{PotentialVar, VarList, parse_target_names};

static VARLIST_CAPACITY: usize = 1000000;

pub fn call_potential_snvs(bam_file: &String,
                           fasta_file: &String,
                           min_alt_count: u32,
                           min_alt_frac: f32,
                           max_coverage: u32,
                           min_mapq: u8)
                           -> VarList {

    let target_names = parse_target_names(&bam_file);

    let bam = bam::Reader::from_path(bam_file).unwrap();
    let mut fasta = fasta::IndexedReader::from_file(&fasta_file).unwrap();

    let mut varlist: Vec<PotentialVar> = Vec::with_capacity(VARLIST_CAPACITY);

    // pileup over all covered sites
    let bases = ['A', 'C', 'G', 'T', 'N'];
    let mut ref_seq: Vec<u8> = vec![];
    let mut prev_tid = 4294967295;

    for p in bam.pileup() {
        let pileup = p.unwrap();
        let tid: usize = pileup.tid() as usize;

        if pileup.depth() > max_coverage {
            continue;
        }

        if tid != prev_tid {
            fasta.read_all(&target_names[tid], &mut ref_seq).expect("Failed to read fasta sequence record.");
        }

        let mut counts = [0; 5]; // A,C,G,T,N

        // pileup the bases for a single position and count number of each base
        for alignment in pileup.alignments() {

            let record = alignment.record();
            // may be faster to implement this as bitwise operation on raw flag in the future?
            if record.mapq() < min_mapq || record.is_unmapped() || record.is_secondary() ||
               record.is_quality_check_failed() ||
               record.is_duplicate() || record.is_supplementary() {
                continue;
            }
            if !alignment.is_del() && !alignment.is_refskip() {

                let base: char = alignment.record().seq()[alignment.qpos().unwrap()] as char;
                let b = match base {
                    'A' | 'a' => 0,
                    'C' | 'c' => 1,
                    'G' | 'g' => 2,
                    'T' | 't' => 3,
                    'N' | 'n' => 4,
                    _ => panic!("Invalid base read from BAM file."),
                };

                counts[b] += 1;
            }
        }

        //let mut var_allele = "N".to_string();
        let mut max_count = 0;
        let mut max_base = 'N';
        //let mut base_cov = 0;
        let ref_allele: String =
            (ref_seq[pileup.pos() as usize] as char).to_string().to_uppercase();

        for i in 0..5 {
            //base_cov += counts[i];
            if counts[i] > max_count && bases[i].to_string() != ref_allele {
                max_count = counts[i];
                max_base = bases[i];
            }
        }

        let alt_frac: f32 = (max_count as f32) / (pileup.depth() as f32); //(max_count as f32) / (base_cov as f32);

        if max_base != 'N' && max_count >= min_alt_count && alt_frac >= min_alt_frac &&
           !(max_base.to_string() == ref_allele) {
            let var_allele = max_base.to_string();

            //println!("A:{};C:{};G:{};T:{};N:{};",
            //         counts[0],
            //         counts[1],
            //         counts[2],
            //         counts[3],
            //         counts[4]);

            //println!("{}\t{}\t{}\t{}",
            //         target_names[tid].clone(),
            //         pileup.pos() + 1,
            //         ref_allele,
            //         var_allele);

            let tid: usize = pileup.tid() as usize;
            let new_var = PotentialVar {
                ix: 0, // this will be set automatically
                chrom: target_names[tid].clone(),
                pos0: pileup.pos() as usize,
                ref_allele: ref_allele,
                var_allele: var_allele,
            };

            varlist.push(new_var);
        }

        prev_tid = tid;
    }
    VarList::new(varlist)
}

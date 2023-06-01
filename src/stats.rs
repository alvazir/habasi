use std::{fmt, time::Instant};

pub(crate) enum StatsUpdateKind {
    Processed,
    Duplicate,
    Merged,
    Replaced,
    ResultUnique,
    ResultMergeableUnique,
    ResultMergeableTotal,
}

macro_rules! make_stats_per_type {
    ($name:ident; $type:ident; $($n:ident),+) => {
        #[derive(Default)]
        pub(crate) struct $name {
            $($n: $type,)+
        }

        #[allow(dead_code)]
        impl $name {
            pub(crate) fn reset(&mut self) {
                $(self.$n = 0;)+
            }

            pub(crate) fn self_check(&self) -> bool {
                (self.processed - self.duplicate - self.merged - self.replaced == self.unique) &&
                    (self.unique + self.mergeable_total == self.total)
            }

            pub(crate) fn add(&mut self, other: &$name) {
                $(self.$n += other.$n;)+
            }

            pub(crate) fn is_empty(&self) -> bool {
                $(if self.$n > 0 { return false };)+
                true
            }

            $(pub(crate) fn $n(&mut self) {
                self.$n += 1;
            }
            )+

            pub(crate) fn grass_filtered_add_count(&mut self, count: usize) {
                self.grass_filtered += count;
            }

            pub(crate) fn decrease_merged(&mut self) {
                self.merged -= 1;
            }

            pub(crate) fn total_string(&self, timer: Instant) -> String {
                macro_rules! empty_if_zero {
                    ($field:ident, $prefix:expr, $suffix:expr) => {
                        let $field = if self.$field > 0 { format!("{}{} {}", $prefix, self.$field, $suffix) } else { String::new() };
                    }
                }
                macro_rules! plugins_count {
                    ($field:ident) => {
                        let $field = if self.$field == 1 { String::from("1 plugin") } else if self.$field > 0 { format!("{} plugins", self.$field) } else { String::new() };
                    }
                }

                plugins_count!(result_plugins);
                plugins_count!(merged_plugins);
                empty_if_zero!(processed, "", "processed");
                empty_if_zero!(duplicate, ", ", "removed(dup)");
                empty_if_zero!(merged, ", ", "merged");
                empty_if_zero!(replaced, ", ", "replaced");
                empty_if_zero!(grass_filtered, ", ", "instances filtered(grass)");
                empty_if_zero!(total, "", "total");
                empty_if_zero!(unique, ", ", "unique");
                empty_if_zero!(mergeable_unique, ", ", "mergeable(unique)");
                empty_if_zero!(mergeable_total, ", ", "mergeable(total)");

                format!("  input({merged_plugins}): {processed}{duplicate}{merged}{replaced}{grass_filtered}\n  output({result_plugins}): {total}{unique}{mergeable_unique}{mergeable_total}, {:.3}s duration", timer.elapsed().as_secs_f64())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                $(if self.$n > 0 && (stringify!($n) == "total" || self.$n != self.total) {write!(f, "{}:{} ", stringify!($n), self.$n)?};)+
                write!(f, "")
            }
        }
    }
}

macro_rules! make_stats {
    ($name:ident; $type:ident; $total:ident; $($n:ident),+) => {
        #[derive(Default)]
        pub(crate) struct $name {
            $total: $type,
            $($n: $type,)+
        }

        impl $name {
            pub(crate) fn reset(&mut self) {
                self.$total.reset();
                $(self.$n.reset();)+
            }

            pub(crate) fn header_adjust(&mut self) {
                self.tes3.decrease_merged();
                self.$total.decrease_merged();
            }

            pub(crate) fn self_check(&self) -> bool {
                self.$total.self_check() $(&& self.$n.self_check())+ && self.total.merged_plugins == self.tes3.processed
            }

            pub(crate) fn add(&mut self, other: &$name) {
                self.$total.add(&other.$total);
                $(self.$n.add(&other.$n);)+
            }

            pub(crate) fn total(&mut self) -> u32 {
                self.$total.total as u32
            }

            pub(crate) fn total_string(&mut self, timer: Instant) -> String {
                format!("{}", self.$total.total_string(timer))
            }

            pub(crate) fn add_merged_plugin(&mut self) {
                self.$total.merged_plugins += 1;
            }

            pub(crate) fn add_result_plugin(&mut self) {
                self.$total.result_plugins += 1;
            }

            pub(crate) fn grass_filtered(&mut self, count: usize) {
                        self.$total.grass_filtered_add_count(count);
            }

            $(pub(crate) fn $n(&mut self, status: StatsUpdateKind) {
                match status {
                    StatsUpdateKind::Processed => {
                        self.$total.processed();
                        self.$n.processed();
                    }
                    StatsUpdateKind::Duplicate => {
                        self.$total.processed();
                        self.$total.duplicate();
                        self.$n.processed();
                        self.$n.duplicate();
                    },
                    StatsUpdateKind::Merged => {
                        self.$total.processed();
                        self.$total.merged();
                        self.$n.processed();
                        self.$n.merged();
                    },
                    StatsUpdateKind::Replaced => {
                        self.$total.processed();
                        self.$total.replaced();
                        self.$n.processed();
                        self.$n.replaced();
                    },
                    StatsUpdateKind::ResultUnique => {
                        self.$total.total();
                        self.$total.unique();
                        self.$n.total();
                        self.$n.unique();
                    },
                    StatsUpdateKind::ResultMergeableUnique => {
                        self.$total.mergeable_unique();
                        self.$n.mergeable_unique();
                    },
                    StatsUpdateKind::ResultMergeableTotal => {
                        self.$total.total();
                        self.$n.total();
                        self.$total.mergeable_total();
                        self.$n.mergeable_total();
                    },
                }
            }
            )+
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "\n\n  [Stats per record type]:\n")?;
                $(if !self.$n.is_empty() { write!(f, "  {}: {}\n", stringify!($n).to_uppercase(), self.$n)?};)+
                write!(f, "")
            }
        }
    }

}

make_stats_per_type!(StatsPerType; usize; merged_plugins, result_plugins, processed, duplicate, merged, replaced, unique, mergeable_unique, mergeable_total, total, grass_filtered);
make_stats!(Stats; StatsPerType; total; tes3, gmst, glob, clas, fact, race, soun, sndg, skil, mgef, scpt, regn, bsgn, sscr, ltex, spel, stat, door, misc, weap, cont, crea, body, ligh, ench, npc_, armo, clot, repa, acti, appa, lock, prob, ingr, book, alch, levi, levc, cell, land, pgrd, dial, info);

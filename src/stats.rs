use crate::increment;
use anyhow::{Context, Result};
use std::{fmt, time::Instant};

#[allow(clippy::module_name_repetitions)]
pub enum StatsUpdateKind {
    Processed,
    Duplicate,
    Merged,
    Replaced,
    ResultUnique,
    ResultMergeableUnique,
    ResultMergeableTotal,
    Excluded,
}

macro_rules! make_stats_per_type {
    ($name:ident; $type:ident; $($n:ident),+) => {
        #[derive(Default)]
        pub struct $name {
            $($n: $type,)+
        }

        #[allow(dead_code)]
        impl $name {
            pub(crate) fn reset(&mut self) {
                $(self.$n = 0;)+
            }

            pub(crate) fn reset_output(&mut self) {
                self.result_plugins = 0;
                self.total = 0;
                self.instances_total = 0;
                self.unique = 0;
                self.mergeable_unique = 0;
                self.mergeable_total = 0;
            }

            pub(crate) fn self_check(&self) -> Result<bool> {
                Ok((self.unique == self.processed
                        .checked_sub(self.duplicate)
                        .and_then(|r| r.checked_sub(self.merged))
                        .and_then(|r| r.checked_sub(self.replaced))
                        .and_then(|r| r.checked_sub(self.excluded))
                        .with_context(|| "Bug: overflow calculating self_check unique")?)
                    && (self.total == self.unique.checked_add(self.mergeable_total).with_context(|| "Bug: overflow calculating self_check total")?))
            }

            pub(crate) fn add(&mut self, other: &$name) -> Result<()> {
                $(self.$n = self.$n.checked_add(other.$n).with_context(|| format!("Bug: overflow adding other.{0} = \"{1}\" to self.{0} = \"{2}\"", stringify!($n), other.$n, self.$n))?;)+
                Ok(())
            }

            pub(crate) fn add_output(&mut self, other: &$name) -> Result<()> {
                macro_rules! add_output {
                    ($field:ident) => {
                        self.$field = self.$field
                            .checked_add(other.$field)
                            .with_context(|| format!(
                                    "Bug: overflow adding other.{0} = \"{1}\" to self.{0} = \"{2}\"",
                                    stringify!($field),
                                    other.$field,
                                    self.$field,
                                    ))?;
                    }
                }
                add_output!(result_plugins);
                add_output!(total);
                add_output!(instances_total);
                add_output!(unique);
                add_output!(mergeable_unique);
                add_output!(mergeable_total);
                Ok(())
            }

            pub(crate) fn substract(&mut self, other: &$name) -> Result<()> {
                $(self.$n = self.$n.checked_sub(other.$n).with_context(|| format!("Bug: overflow subtracting other.{0} = \"{1}\" from self.{0} = \"{2}\"", stringify!($n), other.$n, self.$n))?;)+
                Ok(())
            }

            #[allow(clippy::missing_const_for_fn)]
            pub(crate) fn is_empty(&self) -> bool {
                $(if self.$n > 0 { return false };)+
                true
            }

            #[allow(clippy::arithmetic_side_effects)] // COMMENT: Too expensive(-~2.5% total performance), hardly needed too
            $(pub(crate) fn $n(&mut self) {
                self.$n += 1;
            }
            )+

            pub(crate) fn grass_filtered_add_count(&mut self, count: usize) -> Result<()> {
                Ok(self.grass_filtered = self.grass_filtered.checked_add(count).with_context(|| format!("Bug: overflow adding count = \"{count}\" to grass_filtered"))?)
            }

            pub(crate) fn instances_processed_add_count(&mut self, count: usize) -> Result<()> {
                Ok(self.instances_processed = self.instances_processed.checked_add(count).with_context(|| format!("Bug: overflow adding count = \"{count}\" to instances_processed"))?)
            }

            pub(crate) fn instances_total_add_count(&mut self, count: usize) -> Result<()> {
                Ok(self.instances_total = self.instances_total.checked_add(count).with_context(|| format!("Bug: overflow adding count = \"{count}\" to instances_total"))?)
            }

            pub(crate) fn decrease_merged(&mut self) -> Result<()> {
                Ok(self.merged = self.merged.checked_sub(1).with_context(|| "Bug: overflow decrementing merged")?)
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
                empty_if_zero!(instances_processed, "(", "instances)");
                empty_if_zero!(duplicate, ", ", "removed(dup)");
                empty_if_zero!(merged, ", ", "merged");
                empty_if_zero!(replaced, ", ", "replaced");
                empty_if_zero!(grass_filtered, ", ", "instances filtered(grass)");
                empty_if_zero!(total, "", "total");
                empty_if_zero!(instances_total, "(", "instances)");
                empty_if_zero!(unique, ", ", "unique");
                empty_if_zero!(mergeable_unique, ", ", "mergeable(unique)");
                empty_if_zero!(mergeable_total, ", ", "mergeable(total)");
                empty_if_zero!(excluded, ", ", "excluded");

                format!("  input({merged_plugins}): {processed}{instances_processed}{duplicate}{merged}{replaced}{grass_filtered}\n  output({result_plugins}): {total}{instances_total}{unique}{mergeable_unique}{mergeable_total}{excluded}{}{:.3}s duration", if result_plugins.is_empty() { "" } else { ", " }, timer.elapsed().as_secs_f64())
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
        pub struct $name {
            $total: $type,
            $($n: $type,)+
        }

        impl $name {
            pub(crate) fn reset(&mut self) {
                self.$total.reset();
                $(self.$n.reset();)+
            }

            pub(crate) fn reset_output(&mut self) {
                self.$total.reset_output();
                $(self.$n.reset_output();)+
            }

            pub(crate) fn header_adjust(&mut self) -> Result<()> {
                self.tes3.decrease_merged()?;
                self.$total.decrease_merged()?;
                Ok(())
            }

            pub(crate) fn self_check(&self) -> Result<bool> {
                Ok(self.$total.self_check()? $(&& self.$n.self_check()?)+ && self.total.merged_plugins == self.tes3.processed)
            }

            #[allow(clippy::missing_const_for_fn)]
            pub(crate) fn all_plugins_ignored(&self) -> bool {
                self.total.merged_plugins == 0
            }

            pub(crate) fn add(&mut self, other: &$name) -> Result<()> {
                self.$total.add(&other.$total)?;
                $(self.$n.add(&other.$n)?;)+
                Ok(())
            }

            pub(crate) fn add_output(&mut self, other: &$name) -> Result<()> {
                self.$total.add_output(&other.$total)?;
                $(self.$n.add_output(&other.$n)?;)+
                Ok(())
            }

            pub(crate) fn substract(&mut self, other: &$name) -> Result<()> {
                self.$total.substract(&other.$total)?;
                $(self.$n.substract(&other.$n)?;)+
                Ok(())
            }

            pub(crate) fn total(&mut self) -> Result<u32> {
                u32::try_from(self.$total.total).with_context(|| format!("Bug: failed to cast {}(total, usize) to u32", self.$total.total))
            }

            pub(crate) fn total_string(&mut self, timer: Instant) -> String {
                format!("{}", self.$total.total_string(timer))
            }

            pub(crate) fn add_merged_plugin(&mut self) -> Result<()> {
                self.$total.merged_plugins = increment!(self.$total.merged_plugins);
                Ok(())
            }

            pub(crate) fn add_result_plugin(&mut self) -> Result<()> {
                self.$total.result_plugins = increment!(self.$total.result_plugins);
                Ok(())
            }

            pub(crate) fn grass_filtered(&mut self, count: usize) -> Result<()> {
                        self.$total.grass_filtered_add_count(count)
            }

            pub(crate) fn instances_processed_add_count(&mut self, count: usize) -> Result<()> {
                        self.$total.instances_processed_add_count(count)
            }

            pub(crate) fn instances_total_add_count(&mut self, count: usize) -> Result<()> {
                        self.$total.instances_total_add_count(count)
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
                    StatsUpdateKind::Excluded => {
                        self.$total.excluded();
                        self.$n.excluded();
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

make_stats_per_type!(StatsPerType; usize; merged_plugins, result_plugins, processed, duplicate, merged, replaced, unique, mergeable_unique, mergeable_total, total, excluded, instances_processed, instances_total, grass_filtered);
make_stats!(Stats; StatsPerType; total; tes3, gmst, glob, clas, fact, race, soun, sndg, skil, mgef, scpt, regn, bsgn, sscr, ltex, spel, stat, door, misc, weap, cont, crea, body, ligh, ench, npc_, armo, clot, repa, acti, appa, lock, prob, ingr, book, alch, levi, levc, cell, land, pgrd, dial, info);

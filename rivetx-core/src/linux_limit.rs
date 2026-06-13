#![cfg(unix)]
use anyhow::anyhow;
use log::info;
use serde::Deserialize;

fn default_open_file_limit_soft() -> u64 {
    u16::max_value() as u64
}

fn default_open_file_limit_hard() -> u64 {
    1048576
}

fn default_memlock_rlimit_curr() -> u64 {
    u64::max_value()
}

fn default_memlock_rlimit_max() -> u64 {
    u64::max_value()
}

#[derive(Deserialize, Debug, Clone)]
pub struct Limit {
    #[serde(default = "default_open_file_limit_soft")]
    pub open_file_limit_soft: u64,
    #[serde(default = "default_open_file_limit_hard")]
    pub open_file_limit_hard: u64,
    #[serde(default = "default_memlock_rlimit_curr")]
    pub memlock_rlimit_curr: u64,
    #[serde(default = "default_memlock_rlimit_max")]
    pub memlock_rlimit_max: u64,
}

fn default_limit() -> Limit {
    Limit {
        open_file_limit_soft: default_open_file_limit_soft(),
        open_file_limit_hard: default_open_file_limit_hard(),
        memlock_rlimit_curr: default_memlock_rlimit_curr(),
        memlock_rlimit_max: default_memlock_rlimit_max(),
    }
}

pub fn set_linux_limit(limit: &Limit) -> anyhow::Result<()> {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }

    {
        use rlimit::{setrlimit, Resource};
        let mut soft = limit.open_file_limit_soft;
        let mut hard = limit.open_file_limit_hard;
        if soft > 0 && hard > 0 {
            let mut nofile_rlimit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            let nofile_result = unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, &mut nofile_rlimit) };
            if nofile_result != 0 {
                return Err(anyhow!("Failed to get RLIMIT_NOFILE"));
            }

            println!(
                "nofile_rlimit curr rlim_cur:{:?}, rlim_max:{:?}",
                nofile_rlimit.rlim_cur, nofile_rlimit.rlim_max
            );
            info!(
                "nofile_rlimit curr rlim_cur:{:?}, rlim_max:{:?}",
                nofile_rlimit.rlim_cur, nofile_rlimit.rlim_max
            );

            if soft > nofile_rlimit.rlim_cur || hard > nofile_rlimit.rlim_max {
                if soft < nofile_rlimit.rlim_cur {
                    soft = nofile_rlimit.rlim_cur;
                }

                if hard < nofile_rlimit.rlim_max {
                    hard = nofile_rlimit.rlim_max;
                }

                if soft > hard {
                    soft = hard;
                }

                if soft == u64::max_value() {
                    soft = libc::RLIM_INFINITY;
                }
                if hard == u64::max_value() {
                    hard = libc::RLIM_INFINITY;
                }

                setrlimit(Resource::NOFILE, soft, hard).map_err(|e| {
                    anyhow!("err:setrlimit => soft:{}, hard:{}, e:{}", soft, hard, e)
                })?;

                println!("nofile_rlimit rlim_cur:{:?}, rlim_max:{:?}", soft, hard);
                info!("nofile_rlimit rlim_cur:{:?}, rlim_max:{:?}", soft, hard);
            }
        }
    }

    {
        let mut curr = limit.memlock_rlimit_curr;
        let mut max = limit.memlock_rlimit_max;
        if curr > 0 && max > 0 {
            let mut memlock_rlimit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            let memlock_result =
                unsafe { libc::getrlimit(libc::RLIMIT_MEMLOCK, &mut memlock_rlimit) };
            if memlock_result != 0 {
                return Err(anyhow!("Failed to get RLIMIT_MEMLOCK"));
            }

            println!(
                "memlock_rlimit curr rlim_cur:{:?}, rlim_max:{:?}",
                memlock_rlimit.rlim_cur, memlock_rlimit.rlim_max
            );
            info!(
                "memlock_rlimit curr rlim_cur:{:?}, rlim_max:{:?}",
                memlock_rlimit.rlim_cur, memlock_rlimit.rlim_max
            );

            if curr > memlock_rlimit.rlim_cur || max > memlock_rlimit.rlim_max {
                if curr < memlock_rlimit.rlim_cur {
                    curr = memlock_rlimit.rlim_cur;
                }

                if max < memlock_rlimit.rlim_max {
                    max = memlock_rlimit.rlim_max;
                }

                if curr > max {
                    curr = max;
                }
                let rlimit = libc::rlimit {
                    rlim_cur: if curr == u64::MAX {
                        libc::RLIM_INFINITY
                    } else {
                        curr
                    },
                    rlim_max: if max == u64::MAX {
                        libc::RLIM_INFINITY
                    } else {
                        max
                    },
                };

                if unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlimit) } != 0 {
                    return Err(anyhow!(
                        "Failed to increase rlimit.rlim_cur:{}, rlimit.rlim_max:{}",
                        rlimit.rlim_cur,
                        rlimit.rlim_max
                    ));
                }

                println!(
                    "memlock_rlimit  rlim_cur:{:?}, rlim_max:{:?}",
                    rlimit.rlim_cur, rlimit.rlim_max
                );
                info!(
                    "memlock_rlimit  rlim_cur:{:?}, rlim_max:{:?}",
                    rlimit.rlim_cur, rlimit.rlim_max
                );
            }
        }
    }

    Ok(())
}

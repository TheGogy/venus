use std::{
    fs::{File, create_dir_all},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};

use chess::types::{color::Color, eval::Eval};
use clap::Args;
use engine::{
    position::Position,
    tb::probe::{SyzygyTB, WDL},
    threading::thread::Thread,
    time_management::timecontrol::TimeControl,
    tt::table::TT,
};
use fastrand::Rng;
use humantime::format_duration;

use crate::{
    formats::{marlinfmt::MarlinWDL, virifmt::ViriFmt},
    genfens::gen_random_position,
};

#[derive(Args, Debug)]
pub struct DataGenOpts {
    /// Max starting positions to generate data for.
    #[arg(long, short = 'g', required = true)]
    pub games: usize,

    /// Total nodes to search per move.
    #[arg(long, short = 'n', required = true)]
    pub search_nodes: u64,

    /// Number of threads to use to generate data.
    #[arg(long, short = 't', required = true)]
    pub threads: usize,

    /// Path to syzygy tablebase.
    #[arg(long, short = 's')]
    pub syzygy_path: Option<PathBuf>,

    /// Path to output directory.
    #[arg(long, short = 'o')]
    pub output_path: Option<PathBuf>,

    /// Seed for datagen. Each thread will be seeded separately.
    #[arg(long, short = 'e')]
    pub seed: Option<u64>,

    /// Whether to generate DFRC data.
    #[arg(long, short = 'd', default_value_t = false)]
    pub dfrc: bool,
}

const PRINT_INTERVAL: usize = 64;

// Win/Loss/Draw trackers
static WIN_W: AtomicU64 = AtomicU64::new(0);
static WIN_B: AtomicU64 = AtomicU64::new(0);
static DRAW: AtomicU64 = AtomicU64::new(0);
static WIN_W_ADJ: AtomicU64 = AtomicU64::new(0);
static WIN_B_ADJ: AtomicU64 = AtomicU64::new(0);
static DRAW_ADJ: AtomicU64 = AtomicU64::new(0);

// Adjudication params to fiddle with
const ADJ_RATIO: f32 = 0.9;
const WIN_ADJ_SCORE: Eval = Eval(1500);
const WIN_ADJ_PLIES: usize = 5;
const DRAW_ADJ_SCORE: Eval = Eval(10);
const DRAW_ADJ_PLIES: usize = 10;

// Stop flag
static STOP_ALL: AtomicBool = AtomicBool::new(false);

// Constants to fiddle with
const TT_SIZE: usize = 16;
const RANDOM_MOVES: usize = 6;
const VERIFICATION_NODES: u64 = 75000;
const VERIFICATION_SCORE: Eval = Eval(1000);

pub fn run_datagen(opts: DataGenOpts) -> anyhow::Result<()> {
    ctrlc::set_handler(move || {
        STOP_ALL.store(true, Ordering::Relaxed);
        println!("Stopping datagen after next game end...");
    })
    .expect("Could not setup Ctrl-C handler!");

    let mut tb = SyzygyTB::default();
    let tb_str = opts.syzygy_path.map_or_else(
        || "!!! NO TB! Datagen will produce bad data !!!".to_string(),
        |path| {
            tb.init(path.to_str().unwrap_or(""));
            format!("{}-man tb at {}", tb.max_pcs, path.display())
        },
    );

    let run_id = format!(
        "run_{}_n{}_tb{}_{}",
        chrono::Local::now().format("%Y%m%d-%H%M"),
        opts.search_nodes,
        tb.max_pcs,
        if opts.dfrc { "dfrc" } else { "classical" }
    );

    let outdir = opts.output_path.unwrap_or_else(|| PathBuf::from("data").join(run_id.clone()));
    create_dir_all(&outdir)?;

    let seed = opts.seed.unwrap_or_else(|| chrono::Local::now().timestamp() as u64);

    println!("Id:             {run_id}");
    println!("Seed:           {}", seed);
    println!("Threads:        {}", opts.threads);
    println!("Games:          {}", opts.games);
    println!("Nodes per move: {}", opts.search_nodes);
    println!("Tablebase:      {tb_str}");
    println!("Gen DFRC data:  {}", opts.dfrc);
    println!("Saving to:      {}", outdir.display());

    let tc = TimeControl::FixedNodes(opts.search_nodes);

    let games_per_thread = opts.games / opts.threads;

    std::thread::scope(|s| -> anyhow::Result<()> {
        let mut handles = Vec::new();
        for thread_id in 0..opts.threads {
            let outdir_ref = &outdir;
            handles.push(s.spawn(move || thread_gen_data(thread_id, seed, outdir_ref, games_per_thread, tb, tc, opts.dfrc)));
        }
        for handle in handles {
            handle.join().expect("thread panicked")?;
        }

        Ok(())
    })?;

    Ok(())
}

/// Win/Loss from white's perspective.
/// The STM has no moves and is in check.
fn wdl_result(stm: Color, win_counter: &AtomicU64, loss_counter: &AtomicU64) -> MarlinWDL {
    if stm == Color::White {
        loss_counter.fetch_add(1, Ordering::Relaxed);
        MarlinWDL::Loss
    } else {
        win_counter.fetch_add(1, Ordering::Relaxed);
        MarlinWDL::Win
    }
}

/// TB Win/Loss from white's perspective.
fn tb_wdl(res: WDL, stm: Color) -> MarlinWDL {
    match res {
        WDL::Win => {
            if stm == Color::White {
                MarlinWDL::Win
            } else {
                MarlinWDL::Loss
            }
        }
        WDL::Draw => MarlinWDL::Draw,
        WDL::Loss => {
            if stm == Color::White {
                MarlinWDL::Loss
            } else {
                MarlinWDL::Win
            }
        }
    }
}

/// Updates win/draw adjudication counters, resetting the other on each update.
fn update_adj_counters(abs_score: Eval, win: &mut usize, draw: &mut usize) {
    match abs_score {
        s if s >= WIN_ADJ_SCORE => {
            *win += 1;
            *draw = 0;
        }
        s if s <= DRAW_ADJ_SCORE => {
            *draw += 1;
            *win = 0;
        }
        _ => {
            *win = 0;
            *draw = 0;
        }
    }
}

/// Generate data on this thread.
fn thread_gen_data(
    thread_id: usize,
    seed: u64,
    outdir: &Path,
    games: usize,
    tb: SyzygyTB,
    tc: TimeControl,
    dfrc: bool,
) -> anyhow::Result<()> {
    let mut pos = Position::default();
    let mut rng = Rng::with_seed(seed + thread_id as u64);
    let mut tts = [TT::with_size(TT_SIZE), TT::with_size(TT_SIZE)];

    let tc_verification = TimeControl::FixedNodes(VERIFICATION_NODES);
    let mut thread = Thread::placeholder();

    let outfile = File::create(outdir.join(format!("data_{thread_id}.vf")))?;
    let mut writer = BufWriter::new(outfile);

    let mut positions = 0;

    let start = Instant::now();
    for g in 0..games {
        if thread_id == 0 && g % PRINT_INTERVAL == 0 {
            report_stats(g, games, positions, start);
        }

        if STOP_ALL.load(Ordering::Relaxed) {
            return Ok(());
        }

        gen_random_position(&mut pos, &mut rng, RANDOM_MOVES, dfrc);
        for tt in tts.iter_mut() {
            tt.clear();
        }

        thread.tm.set_tc(tc_verification, pos.stm());
        thread.prepare_search(pos.board.state.halfmoves);
        pos.iterative_deepening::<false>(&mut thread, &tts[pos.stm().idx()], &tb);
        if thread.eval.abs() >= VERIFICATION_SCORE {
            continue;
        }

        let mut compressed_pos = ViriFmt::new(&pos.board);
        let should_adj = rng.f32() < ADJ_RATIO;
        let mut win_adj_counter = 0;
        let mut draw_adj_counter = 0;

        let result = loop {
            // Checkmate / stalemate.
            if !pos.board.has_moves() {
                break if pos.board.in_check() {
                    wdl_result(pos.stm(), &WIN_W, &WIN_B)
                } else {
                    DRAW.fetch_add(1, Ordering::Relaxed);
                    MarlinWDL::Draw
                };
            }

            if pos.board.is_draw(thread.ply) {
                DRAW.fetch_add(1, Ordering::Relaxed);
                break MarlinWDL::Draw;
            }

            if tb.can_probe(&pos.board)
                && let Some(res) = tb.probe_wdl(&pos.board)
            {
                break tb_wdl(res, pos.stm());
            }

            // Make next move and continue loop.
            tts[pos.stm().idx()].increment_age();
            thread.tm.set_tc(tc, pos.stm());
            thread.prepare_search(pos.board.state.halfmoves);
            pos.reinit_nnue();
            pos.iterative_deepening::<false>(&mut thread, &tts[pos.stm().idx()], &tb);

            let best_move = thread.best_move();
            let score = thread.eval;
            compressed_pos.push(&pos.board, best_move, score);

            // If either side proves mate, terminate immediately.
            if score.is_terminal() {
                break wdl_result(pos.stm(), &WIN_W, &WIN_B);
            }

            // Adjudicate if we should (let some games play out until mate is proven).
            if should_adj {
                let abs_score = score.abs();
                update_adj_counters(abs_score, &mut win_adj_counter, &mut draw_adj_counter);

                if win_adj_counter >= WIN_ADJ_PLIES {
                    break wdl_result(pos.stm(), &WIN_W_ADJ, &WIN_B_ADJ);
                }
                if draw_adj_counter >= DRAW_ADJ_PLIES {
                    DRAW_ADJ.fetch_add(1, Ordering::Relaxed);
                    break MarlinWDL::Draw;
                }
            }

            pos.make_move(best_move, &mut thread);
        };

        positions += compressed_pos.moves.len();

        compressed_pos.finish(result);
        compressed_pos.write(&mut writer)?;
    }

    writer.flush()?;

    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn report_stats(completed: usize, total: usize, positions: usize, start: Instant) {
    if completed == 0 {
        return;
    }

    let elapsed = start.elapsed();
    let time_per_game = elapsed.as_secs_f64() / completed as f64;
    let remaining_games = total.saturating_sub(completed);
    let eta_secs = remaining_games as f64 * time_per_game;
    let time_remaining = format_duration(Duration::from_secs_f64(eta_secs)).to_string();

    let pos_per_sec = positions as f64 / elapsed.as_secs_f64();

    let (ww, wa) = (WIN_W.load(Ordering::Relaxed), WIN_W_ADJ.load(Ordering::Relaxed));
    let (bw, ba) = (WIN_B.load(Ordering::Relaxed), WIN_B_ADJ.load(Ordering::Relaxed));
    let (dr, da) = (DRAW.load(Ordering::Relaxed), DRAW_ADJ.load(Ordering::Relaxed));

    let t_w = ww + wa;
    let t_b = bw + ba;
    let t_d = dr + da;
    let t_adj = wa + ba + da;
    let sum = t_w + t_b + t_d;

    let pct = |val: u64| (val as f64 / sum as f64) * 100.0;
    let progress = (completed as f64 / total as f64) * 100.0;

    println!(
        "[{}] - GAMES={:<8} ({:>6.2}%) | PPS {:<3.2} | ETA={:>40} | WW={:<7} ({:>5.2}%) | BW={:<7} ({:>5.2}%) | DR={:<7} ({:>5.2}%) | ADJ={:<7}",
        chrono::Local::now().format("%d-%H%M"),
        completed,
        progress,
        pos_per_sec,
        time_remaining,
        t_w,
        pct(t_w),
        t_b,
        pct(t_b),
        t_d,
        pct(t_d),
        t_adj
    );
}

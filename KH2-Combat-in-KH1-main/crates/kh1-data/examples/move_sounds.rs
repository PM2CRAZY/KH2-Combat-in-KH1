//! Research (the user: Explosion's and Magnet Burst's sounds don't sound right): renders what
//! KH2 plays for each move from the user's own files, as WAVs to listen to (local only).
//! Per OpenKH's motion-trigger table: frame trigger 1 starts a PAX element (its sound), 8 plays
//! a sound (package = bank, sound = index) from the Keyblade's motion set, 14 plays a voice
//! (`00battle` VTBL: Sora's id → a choice of voice clips in `voice/us/battle/*_sora`).
//!
//! `cargo run -p kh1-data --example move_sounds -- <dir with se_se999/se_se000/se_se500/voice scd> <out dir> [one|zero]`
use kh2_formats::scd::Scd;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = std::path::Path::new(&a[1]);
    let out = std::path::Path::new(&a[2]);
    // Voice table values counted from 1 (or 0).
    let one = a.get(3).map_or(true, |s| s != "zero");
    let bank = |n: u32| std::fs::read(dir.join(format!("se_se{n:03}.win32.scd"))).unwrap();
    let banks: Vec<(u32, Vec<u8>)> = vec![(0, bank(0)), (500, bank(500)), (999, bank(999))];
    let voice = std::fs::read(dir.join("voice_us_battle_tt0_sora.win32.scd")).unwrap();
    let vscd = Scd::parse(&voice).unwrap();
    let sound = |id: u32| -> (u32, u16, Vec<i16>) {
        let (b, i) = (id / 1000, (id % 1000) as usize);
        let bytes = &banks.iter().find(|x| x.0 == b).unwrap().1;
        let p = Scd::parse(bytes).unwrap().sound_id(i).unwrap();
        (p.rate, p.channels, p.samples.iter().map(|x| (*x as f32 * p.volume) as i16).collect())
    };
    let voice_clip = |v: i32| -> (u32, u16, Vec<i16>) {
        let _ = one;
        let p = vscd.sound_id(v as usize).unwrap();
        (p.rate, p.channels, p.samples.iter().map(|x| (*x as f32 * p.volume) as i16).collect())
    };
    // (frame, what)
    enum S {
        Se(u32),
        Voice(i32),
    }
    let moves: Vec<(&str, Vec<(u32, S)>)> = vec![
        // Explosion (motion 166): voice 6 (17), elements 44 (999038), 48 (999042), Keyblade 500/42.
        ("explosion", vec![(0, S::Voice(17)), (0, S::Se(999038)), (18, S::Se(500042)), (34, S::Se(999042))]),
        // Magnet Burst (197): voice 7 (12) at 0, element 51 (169) at 10, Keyblade 500/44 at 18,
        // voice 6 (18) at 50, element 52 (999021) at 60.
        ("magnet_burst", vec![(0, S::Voice(12)), (10, S::Se(169)), (18, S::Se(500044)), (50, S::Voice(18)), (60, S::Se(999021))]),
    ];
    std::fs::create_dir_all(out).unwrap();
    let rate = 48000u32;
    for (name, events) in moves {
        let mut mix: Vec<f32> = vec![0.0; rate as usize * 6 * 2];
        for (frame, s) in &events {
            let (r, ch, data) = match s {
                S::Se(id) => sound(*id),
                S::Voice(v) => voice_clip(*v),
            };
            let label = match s {
                S::Se(id) => format!("se_{id}"),
                S::Voice(v) => format!("voice_{v}{}", if one { "" } else { "_zero" }),
            };
            write(&out.join(format!("{name}_{label}.wav")), r, ch, &data);
            let start = (*frame as f32 / 60.0 * rate as f32) as usize * 2;
            let frames = data.len() / ch as usize;
            for k in 0..frames {
                let t = (k as f32 * rate as f32 / r as f32) as usize;
                let (l, rr) = if ch == 1 { (data[k], data[k]) } else { (data[k * 2], data[k * 2 + 1]) };
                if let Some(x) = mix.get_mut(start + t * 2) {
                    *x += l as f32;
                }
                if let Some(x) = mix.get_mut(start + t * 2 + 1) {
                    *x += rr as f32;
                }
            }
            println!("{name}: frame {frame}: {label}, {r} Hz, {ch} ch, {:.2} s", frames as f32 / r as f32);
        }
        let end = mix.iter().rposition(|x| *x != 0.0).unwrap_or(0) + 2;
        let pcm: Vec<i16> = mix[..end.min(mix.len())].iter().map(|x| x.clamp(-32768.0, 32767.0) as i16).collect();
        write(&out.join(format!("{name}_kh2.wav")), rate, 2, &pcm);
    }
}

fn write(path: &std::path::Path, rate: u32, ch: u16, data: &[i16]) {
    let bytes: Vec<u8> = data.iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut w = Vec::new();
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36 + bytes.len() as u32).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&ch.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * ch as u32 * 2).to_le_bytes());
    w.extend_from_slice(&(ch * 2).to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    w.extend_from_slice(&bytes);
    std::fs::write(path, w).unwrap();
}

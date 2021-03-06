use av_metrics::video::*;
use clap::{App, Arg};
use console::style;
use serde::Serialize;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Stdout, Write};
use std::path::Path;

fn main() -> Result<(), String> {
    let cli = App::new("AV Metrics")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("INPUT1")
                .help("The first input file to compare--currently supports Y4M files")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("INPUT2")
                .help("The second input file to compare--order does not matter")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("METRIC")
                .help("Run only one metric, instead of the entire suite")
                .long("metric")
                .takes_value(true)
                .possible_value("psnr")
                .possible_value("apsnr")
                .possible_value("psnrhvs")
                .possible_value("ssim")
                .possible_value("msssim")
                .possible_value("ciede2000"),
        )
        .arg(
            Arg::with_name("JSON")
                .help("Output results as JSON--useful for piping to other programs")
                .long("export-json")
                .takes_value(true)
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("CSV")
                .help("Output results as CSV")
                .long("export-csv")
                .takes_value(true)
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("MARKDOWN")
                .help("Output results as Markdown")
                .long("export-markdown")
                .takes_value(true)
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("FILE")
                .help("Output results to a file")
                .long("export-file")
                .takes_value(true)
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("QUIET")
                .help("Do not output to stdout")
                .long("quiet")
                .takes_value(false),
        )
        .get_matches();
    let input1 = cli.value_of("INPUT1").unwrap();
    let input2 = cli.value_of("INPUT2").unwrap();
    let input_type1 = InputType::detect(input1);
    let input_type2 = InputType::detect(input2);
    let mut writers = vec![];
    if let Some(filename) = cli.value_of("FILE") {
        writers.push(OutputType::TEXT(BufWriter::new(
            File::create(filename).map_err(|err| err.to_string())?,
        )));
    };
    if let Some(filename) = cli.value_of("JSON") {
        writers.push(OutputType::JSON(BufWriter::new(
            File::create(filename).map_err(|err| err.to_string())?,
        )));
    };
    if let Some(filename) = cli.value_of("CSV") {
        writers.push(OutputType::CSV(BufWriter::new(
            File::create(filename).map_err(|err| err.to_string())?,
        )));
    };
    if let Some(filename) = cli.value_of("MARKDOWN") {
        writers.push(OutputType::Markdown(BufWriter::new(
            File::create(filename).map_err(|err| err.to_string())?,
        )));
    };
    if !cli.is_present("QUIET") {
        writers.push(OutputType::Stdout(BufWriter::new(std::io::stdout())));
    }

    match (input_type1, input_type2) {
        (InputType::Video(c1), InputType::Video(c2)) => {
            run_video_metrics(input1, c1, input2, c2, &mut writers, cli.value_of("METRIC"))
        }
        (InputType::Audio(_c1), InputType::Audio(_c2)) => {
            Err("No audio metrics currently implemented, exiting.".to_owned())
        }
        (InputType::Video(_), InputType::Audio(_)) | (InputType::Audio(_), InputType::Video(_)) => {
            Err("Incompatible input files.".to_owned())
        }
        (InputType::Unknown, _) | (_, InputType::Unknown) => {
            Err("Unsupported input format.".to_owned())
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum InputType {
    Video(VideoContainer),
    #[allow(dead_code)]
    Audio(AudioContainer),
    Unknown,
}

impl InputType {
    pub fn detect<P: AsRef<Path>>(filename: P) -> Self {
        let ext = filename
            .as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        match ext.to_lowercase().as_str() {
            "y4m" => InputType::Video(VideoContainer::Y4M),
            _ => InputType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum VideoContainer {
    Y4M,
}

impl VideoContainer {
    // TODO: Actually be generic and support more input types
    pub fn get_decoder<'d>(self, file: &'d mut File) -> y4m::Decoder<&'d mut File> {
        match self {
            VideoContainer::Y4M => y4m::Decoder::new(file).expect("Failed to read y4m file"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum AudioContainer {
    // Coming soon
}

#[derive(Debug, Clone, Copy, Serialize, Default)]
struct MetricsResults {
    #[serde(skip_serializing_if = "Option::is_none")]
    psnr: Option<PlanarMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    apsnr: Option<PlanarMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    psnr_hvs: Option<PlanarMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ssim: Option<PlanarMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    msssim: Option<PlanarMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ciede2000: Option<f64>,
}

fn run_video_metrics<P: AsRef<Path>>(
    input1: P,
    container1: VideoContainer,
    input2: P,
    container2: VideoContainer,
    writers: &mut Vec<OutputType>,
    metric: Option<&str>,
) -> Result<(), String> {
    let mut results = MetricsResults::default();

    if metric.is_none() || metric == Some("psnr") {
        results.psnr = Psnr::run(input1.as_ref(), container1, input2.as_ref(), container2);
    }

    if metric.is_none() || metric == Some("apsnr") {
        results.apsnr = APsnr::run(input1.as_ref(), container1, input2.as_ref(), container2);
    }

    if metric.is_none() || metric == Some("psnrhvs") {
        results.psnr_hvs = PsnrHvs::run(input1.as_ref(), container1, input2.as_ref(), container2);
    }

    if metric.is_none() || metric == Some("ssim") {
        results.ssim = Ssim::run(input1.as_ref(), container1, input2.as_ref(), container2);
    }

    if metric.is_none() || metric == Some("msssim") {
        results.msssim = MsSsim::run(input1.as_ref(), container1, input2.as_ref(), container2);
    }

    if metric.is_none() || metric == Some("ciede2000") {
        results.ciede2000 =
            Ciede2000::run(input1.as_ref(), container1, input2.as_ref(), container2);
    }

    for writer in writers.iter_mut() {
        match writer {
            OutputType::JSON(writer) => {
                writeln!(writer, "{}", serde_json::to_string(&results).unwrap())
                    .map_err(|err| err.to_string())?;
            }
            OutputType::CSV(_) => {
                writeln!(writer, "Metric;Y;U/Cb;V/Cr;Avg;Delta").map_err(|err| err.to_string())?;
                CSV::print_result(writer, "PSNR", results.psnr)?;
                CSV::print_result(writer, "APSNR", results.apsnr)?;
                CSV::print_result(writer, "PSNR HVS", results.psnr_hvs)?;
                CSV::print_result(writer, "SSIM", results.ssim)?;
                CSV::print_result(writer, "MSSSIM", results.msssim)?;
                CSV::print_result(writer, "CIEDE2000", results.ciede2000)?;
            }
            OutputType::Markdown(_) => {
                match metric {
                    Some(metr) => {
                        writeln!(
                            writer,
                            "  Computing metric for: {} using the YUV/YCbCr system...",
                            metr,
                        )
                        .map_err(|err| err.to_string())?
                    }
                    None => {
                        writeln!(
                            writer,
                            "  Computing metrics for: PSNR, APSNR, PSNR-HVS, SSIM, MSSIM, CIEDE2000 using the YUV/YCbCr system...",
                        )
                        .map_err(|err| err.to_string())?
                    }
                };

                writeln!(
                    writer,
                    "    Results for comparing {} to {}: \n",
                    input1.as_ref().display(),
                    input2.as_ref().display()
                )
                .map_err(|err| err.to_string())?;

                Markdown::print_result(writer, "PSNR", results.psnr)?;
                Markdown::print_result(writer, "APSNR", results.apsnr)?;
                Markdown::print_result(writer, "PSNR HVS", results.psnr_hvs)?;
                Markdown::print_result(writer, "SSIM", results.ssim)?;
                Markdown::print_result(writer, "MSSSIM", results.msssim)?;
                Markdown::print_result(writer, "CIEDE2000", results.ciede2000)?;
            }
            OutputType::Stdout(_) | OutputType::TEXT(_) => {
                match metric {
                    Some(metr) => writeln!(
                        writer,
                        "  {} metric for: {} using the {} system...",
                        style("Computing").yellow(),
                        style(metr).cyan(),
                        style("YUV/YCbCr").magenta()
                    )
                    .map_err(|err| err.to_string())?,
                    None => writeln!(
                        writer,
                        "  {} metrics for: {}, {}, {}, {}, {}, {} using the {} system...",
                        style("Computing").yellow(),
                        style("PSNR").cyan(),
                        style("APSNR").cyan(),
                        style("PSNR-HVS").cyan(),
                        style("SSIM").cyan(),
                        style("MSSIM").cyan(),
                        style("CIEDE2000").cyan(),
                        style("YUV/YCbCr").magenta()
                    )
                    .map_err(|err| err.to_string())?,
                };

                writeln!(
                    writer,
                    "    {} for comparing {} to {}: \n",
                    style("Results").yellow(),
                    style(input1.as_ref().display()).italic().cyan(),
                    style(input2.as_ref().display()).italic().cyan()
                )
                .map_err(|err| err.to_string())?;
                Text::print_result(writer, "PSNR", results.psnr)?;
                Text::print_result(writer, "APSNR", results.apsnr)?;
                Text::print_result(writer, "PSNR HVS", results.psnr_hvs)?;
                Text::print_result(writer, "SSIM", results.ssim)?;
                Text::print_result(writer, "MSSSIM", results.msssim)?;
                Text::print_result(writer, "CIEDE2000", results.ciede2000)?;
            }
        }
    }
    Ok(())
}

enum OutputType {
    JSON(BufWriter<File>),
    CSV(BufWriter<File>),
    Markdown(BufWriter<File>),
    TEXT(BufWriter<File>),
    Stdout(BufWriter<Stdout>),
}

impl Write for OutputType {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            OutputType::JSON(f)
            | OutputType::CSV(f)
            | OutputType::Markdown(f)
            | OutputType::TEXT(f) => f.write(buf),
            OutputType::Stdout(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            OutputType::JSON(f)
            | OutputType::CSV(f)
            | OutputType::Markdown(f)
            | OutputType::TEXT(f) => f.flush(),
            OutputType::Stdout(s) => s.flush(),
        }
    }
}

trait CliMetric {
    type VideoResult: Serialize;

    fn run<P: AsRef<Path>>(
        input1: P,
        container1: VideoContainer,
        input2: P,
        container2: VideoContainer,
    ) -> Option<Self::VideoResult> {
        let mut file1 = File::open(input1).expect("Failed to open input file 1");
        let mut file2 = File::open(input2).expect("Failed to open input file 2");
        let mut dec1 = container1.get_decoder(&mut file1);
        let mut dec2 = container2.get_decoder(&mut file2);
        Self::calculate_video_metric(&mut dec1, &mut dec2).ok()
    }

    fn calculate_video_metric<D: Decoder>(
        dec1: &mut D,
        dec2: &mut D,
    ) -> Result<Self::VideoResult, Box<dyn Error>>;
}

struct Psnr;

impl CliMetric for Psnr {
    type VideoResult = PlanarMetrics;

    fn calculate_video_metric<D: Decoder>(
        dec1: &mut D,
        dec2: &mut D,
    ) -> Result<Self::VideoResult, Box<dyn Error>> {
        psnr::calculate_video_psnr(dec1, dec2, None)
    }
}

struct APsnr;

impl CliMetric for APsnr {
    type VideoResult = PlanarMetrics;

    fn calculate_video_metric<D: Decoder>(
        dec1: &mut D,
        dec2: &mut D,
    ) -> Result<Self::VideoResult, Box<dyn Error>> {
        psnr::calculate_video_apsnr(dec1, dec2, None)
    }
}

struct PsnrHvs;

impl CliMetric for PsnrHvs {
    type VideoResult = PlanarMetrics;

    fn calculate_video_metric<D: Decoder>(
        dec1: &mut D,
        dec2: &mut D,
    ) -> Result<Self::VideoResult, Box<dyn Error>> {
        psnr_hvs::calculate_video_psnr_hvs(dec1, dec2, None)
    }
}

struct Ssim;

impl CliMetric for Ssim {
    type VideoResult = PlanarMetrics;

    fn calculate_video_metric<D: Decoder>(
        dec1: &mut D,
        dec2: &mut D,
    ) -> Result<Self::VideoResult, Box<dyn Error>> {
        ssim::calculate_video_ssim(dec1, dec2, None)
    }
}

struct MsSsim;

impl CliMetric for MsSsim {
    type VideoResult = PlanarMetrics;

    fn calculate_video_metric<D: Decoder>(
        dec1: &mut D,
        dec2: &mut D,
    ) -> Result<Self::VideoResult, Box<dyn Error>> {
        ssim::calculate_video_msssim(dec1, dec2, None)
    }
}

struct Ciede2000;

impl CliMetric for Ciede2000 {
    type VideoResult = f64;

    fn calculate_video_metric<D: Decoder>(
        dec1: &mut D,
        dec2: &mut D,
    ) -> Result<Self::VideoResult, Box<dyn Error>> {
        ciede::calculate_video_ciede(dec1, dec2, None)
    }
}

trait PrintResult<T> {
    fn print_result(writer: &mut OutputType, header: &str, result: Option<T>)
        -> Result<(), String>;
}

struct Text;

impl PrintResult<PlanarMetrics> for Text {
    fn print_result(
        writer: &mut OutputType,
        header: &str,
        result: Option<PlanarMetrics>,
    ) -> Result<(), String> {
        if let Some(result) = result {
            writeln!(
                writer,
                "     {:<10} →  Y: {:<8.4} U/Cb: {:<8.4} V/Cr: {:<8.4} Avg value: {:<8.4}",
                style(header).cyan(),
                result.y,
                result.u,
                result.v,
                result.avg
            )
            .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

impl PrintResult<f64> for Text {
    fn print_result(
        writer: &mut OutputType,
        header: &str,
        result: Option<f64>,
    ) -> Result<(), String> {
        if let Some(result) = result {
            writeln!(
                writer,
                "     {:<10} →  Delta: {:<8.4}",
                style(header).cyan(),
                result
            )
            .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

struct CSV;

impl PrintResult<PlanarMetrics> for CSV {
    fn print_result(
        writer: &mut OutputType,
        header: &str,
        result: Option<PlanarMetrics>,
    ) -> Result<(), String> {
        if let Some(result) = result {
            writeln!(
                writer,
                "{};{};{};{};{};",
                header, result.y, result.u, result.v, result.avg,
            )
            .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

impl PrintResult<f64> for CSV {
    fn print_result(
        writer: &mut OutputType,
        header: &str,
        result: Option<f64>,
    ) -> Result<(), String> {
        if let Some(delta) = result {
            writeln!(writer, "{};;;;;{}", header, delta).map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

struct Markdown;

impl PrintResult<PlanarMetrics> for Markdown {
    fn print_result(
        writer: &mut OutputType,
        header: &str,
        result: Option<PlanarMetrics>,
    ) -> Result<(), String> {
        if let Some(result) = result {
            writeln!(
                writer,
                "* {}\n    * Y: {}\n    * U/Cb:{}\n    * V/Cr:{}\n    Avg value:* {}",
                header, result.y, result.u, result.v, result.avg,
            )
            .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

impl PrintResult<f64> for Markdown {
    fn print_result(
        writer: &mut OutputType,
        header: &str,
        result: Option<f64>,
    ) -> Result<(), String> {
        if let Some(delta) = result {
            writeln!(writer, "* {}\n    * Delta: {}", header, delta)
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }
}

/// Functions and types used to manage a task responsible for managing sample data.
use comm::{Chan, Port};
use core::io::{WriterUtil};
use RingBuffer = ring_buffer::RingBuffer;
use runits::generated::*;
use runits::units::*;
use task_runner::*;

pub enum Msg
{
	AddSample(~str, float, uint),							// sample set name + sample value, number of samples to retain
	GetSampleSet(~str, Chan<(~RingBuffer, uint)>),		// sample set name + channel which receives a copy of the buffer and num (global) adds
	GetSampleSets(~[~str], Chan<~[~RingBuffer]>),	// sample set names + channel which receives a copy of the buffers
	ExitMsg,
}

pub fn manage_samples(port: comm::Port<Msg>)
{
	let sample_sets = std::map::HashMap();
	let mut num_adds = 0;	// this is for a hack in build_sparkline
	
	loop
	{
		match comm::recv(port)
		{
			AddSample(copy name, value, capacity) =>
			{
				let name = @name;
				if !sample_sets.contains_key(name)
				{
					sample_sets.insert(name, @~RingBuffer(capacity));
				}
				
				let buffer = sample_sets[name];
				buffer.push(value);
				num_adds += 1;
			}
			GetSampleSet(copy name, ch) =>
			{
				let buffer = sample_sets[@name];
				ch.send((copy *buffer, num_adds));
			}
			GetSampleSets(ref names, ch) =>
			{
				let buffers = do names.map |n| {copy *sample_sets[@copy n]};
				ch.send(buffers);
			}
			ExitMsg =>
			{
				break;
			}
		}
	}
}

pub struct Chart
{
	path: ~str,					// path to the generated png file
	sample_sets: ~[~str],		// name each sample set was saved under
	legends: ~[~str],			// name to use in the legend for each sample set
	interval: float,				// in seconds
	units: Unit,					// Unit the samples were saved with
	title: ~str,					// main title
	y_label: ~str,				// x label is assumed to be Time
}

// Creates a chart with color-coded lines for each sample set. Id is used to 
// uniquely identify the charts (across tasks). Note that multiple charts are
// normally created at once to amortize the cost of starting up Rscript. 
//
// It would be more efficient to create these charts client-side. It might also
// be more flexible (easier to allow users to customize their appearence).
// However we'd have to use some javascript library to create the charts
// instead of R and it's very hard for any tool to compete with R's chart
// capabilities.
pub fn create_charts(id: ~str, charts: &[Chart], samples_chan: Chan<Msg>)
{
	let mut script = ~"";
	
	// Assemble a mondo R script,
	let port = Port();
	let chan = Chan(port);
	for charts.each |chart|
	{
		assert chart.sample_sets.is_not_empty();
		
		samples_chan.send(samples::GetSampleSets(copy chart.sample_sets, chan));
		let samples = port.recv();
		append_r_script(chart, samples, &mut script);
	}
	
	// and execute it.
	let action: JobFn =
		|move script, copy id|
		{
			let path = path::from_str(fmt!("/tmp/gnos-%s.R", id));
			match io::file_writer(&path, ~[io::Create, io::Truncate])
			{
				result::Ok(writer) =>
				{
					writer.write_str(script);
					run_script(&path)
				}
				result::Err(ref err) =>
				{
					option::Some(fmt!("Failed to create %s: %s", path.to_str(), *err))
				}
			}
		};
	let cleanup: ExitFn = || {};
	run(Job {action: action, policy: IgnoreFailures}, ~[cleanup]);
}

// The poll interval can vary by quite a bit so using seconds for the x axis
// will often be hard to interpret. So we use this function to choose a time
// unit that should be suitable for the times in use.
priv fn get_time_interval(interval: float, num_samples: uint) -> (float, ~str)
{
	let max_time = interval*(num_samples as float - 1.0);
	let x = from_units(max_time, Second).normalize_time();
	(interval*(x.value/max_time), x.units.to_str())
}

// This is similar to get_time_interval: we want to use the best units we can to
// represent all of our sample values. Note that this assumes that the samples
// were originaly in kbps. TODO: seems a bit error prone to make that
// assumption.
priv fn get_value_scaling(samples: ~[~RingBuffer]) -> (float, ~str)
{
	let max_values = do samples.map |s| {iter::max(*s)};
	let max_value = max_values.max();
	
	let x = from_units(max_value, Kilo*Bit/Second).normalize_si();
	(x.value/max_value, x.units.to_str())
}

// We generate the R script instead of using mustache because the mustache
// version winds up being all templates anyway.
//
// Files wind up looking like this:
// library(RColorBrewer)
// png('/Users/jessejones/Source/gnos/html/generated/10.101.0.2-in-interfaces.png', 600, 400)
// 
// samples1 = c(11.7057, 11.6448, 11.6452, 11.6578, 11.6615, 11.6394, 11.6578, 11.6732, 11.6452, 11.6461, 11.6732, 11.6278, 11.6636, 11.6615, 11.6452)
// samples2 = c(1.6499, 0.8344, 0.8792, 1.6216, 0.796, 0.8236, 1.6200, 0.8168, 0.8608, 1.5848, 0.7968, 0.8228, 1.6208, 0.8328, 0.824)
// samples3 = c(0.9099, 0.4196, 0.4656, 0.9403, 0.4196, 0.4654, 0.9403, 0.42, 0.4656, 0.9114, 0.42, 0.4649, 0.9408, 0.4196, 0.4656)
// samples4 = c(1.6145, 0.8511, 0.8072, 1.6752, 0.796, 0.8236, 1.6032, 0.8152, 0.824, 1.6048, 0.8136, 0.8060, 1.6376, 0.8144, 0.824)
// samples5 = c(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
// times = -c(14*0.3333, 13*0.3333, 12*0.3333, 11*0.3333, 10*0.3333, 9*0.3333, 8*0.3333, 7*0.3333, 6*0.3333, 5*0.3333, 4*0.3333, 3*0.3333, 2*0.3333, 1*0.3333, 0*0.3333)
// 
// max_samples1 = max(samples1)
// max_samples2 = max(samples2)
// max_samples3 = max(samples3)
// max_samples4 = max(samples4)
// max_samples5 = max(samples5)
// max_samples = max(c(max_samples1, max_samples2, max_samples3, max_samples4, max_samples5))
// 
// colors = brewer.pal(5, 'Set1')
// plot(samples1 ~ times, type = 'l', lwd = 2, col = colors[1], ylim = c(0, max_samples), xlab = 'Time (min)', ylab = 'Bandwidth (kb/s)', main = '10.101.0.2 In Bandwidth')
// points(samples2 ~ times, type = 'l', lwd = 2, col = colors[2])
// points(samples3 ~ times, type = 'l', lwd = 2, col = colors[3])
// points(samples4 ~ times, type = 'l', lwd = 2, col = colors[4])
// points(samples5 ~ times, type = 'l', lwd = 2, col = colors[5])
// 
// legend('topleft', c('eth0', 'eth1', 'eth2', 'eth3', 'lo'), fill = c(colors[1], colors[2], colors[3], colors[4], colors[5]))
// grid()
// 
// dev.off()
priv fn append_r_script(chart: &Chart, samples: ~[~RingBuffer], script: &mut ~str)
{
	let num_lines = samples.len();
	let num_samples = samples[0].len();
	
	// The built in palettes don't do a very good job of picking colors that are both
	// pleasing to the eye and visually distinct. There are a number of packages
	// that provide better palettes. RColorBrewer seems to be one of the better
	// ones and has a really nifty online palette picker at http://colorbrewer2.org
	// Manual is at http://svitsrv25.epfl.ch/R-doc/library/RColorBrewer/html/ColorBrewer.html
	if script.len() == 0
	{
		*script += "library(RColorBrewer)\n";
	}
	else
	{
		*script += "\n####################################################\n";
	}
	*script += fmt!("png('%s', 600, 400)\n\n", chart.path);
	
	let (scaling, y_units) = get_value_scaling(samples);
	for samples.eachi |i, buffer|
	{
		assert buffer.len() == num_samples;
		let values = do iter::map_to_vec(*buffer) |x| {(scaling*x).to_str()};
		*script += fmt!("samples%? = c(%s)\n", i+1, str::connect(values, ", "));
	}
	
	let (interval, x_units) = get_time_interval(chart.interval, num_samples);
	let times = do vec::from_fn(num_samples) |i|
	{
		fmt!("%?*%?", num_samples - i - 1, interval)
	};
	*script += fmt!("times = -c(%s)\n\n", str::connect(times, ", "));
	
	for num_lines.timesi |i|
	{
		*script += fmt!("max_samples%? = max(samples%?)\n", i+1, i+1);
	}
	let max_samples = do vec::from_fn(num_lines) |i| {fmt!("max_samples%?", i+1)};
	*script += fmt!("max_samples = max(c(%s))\n\n", str::connect(max_samples, ", "));
	
	*script += fmt!("colors = brewer.pal(%?, 'Set1')\n", num_lines);
	*script += fmt!("plot(samples1 ~ times, type = 'l', lwd = 2, col = colors[1], ylim = c(0, max_samples), xlab = 'Time (%s)', ylab = '%s (%s)', main = '%s')\n", x_units, chart.y_label, y_units, chart.title);
	for uint::iterate(1, num_lines) |i|
	{
		// Note that R vector indexing is 1-based.
		*script += fmt!("points(samples%? ~ times, type = 'l', lwd = 2, col = colors[%?])\n", i+1, i+1);
	};
	*script += "\n";
	
	let legends = do chart.legends.map |n| {fmt!("'%s'", n)};
	let colors = do vec::from_fn(num_lines) |i| {fmt!("colors[%?]", i + 1)};
	*script += fmt!("legend('topleft', c(%s), fill = c(%s))\n", str::connect(legends, ", "), str::connect(colors, ", "));
	*script += "grid()\n\n";
	
	*script += "dev.off()\n";
}

priv fn run_script(path: &Path) -> option::Option<~str>
{
	fn get_output(label: &str, reader: io::Reader) -> ~str
	{
		let text = str::from_bytes(reader.read_whole_stream());
		if text.is_not_empty() {fmt!("%s:\n%s\n", label, text)} else {~""}
	}
	
	let program = core::run::start_program("Rscript", [path.to_str()]);
	let result = program.finish();
	if result != 0
	{
		let mut err = fmt!("Rscript %s returned %?\n", path.to_str(), result);
		err += get_output("stdout", program.output());
		err += get_output("stderr", program.err());
		option::Some(err)
	}
	else
	{
		option::None
	}
}

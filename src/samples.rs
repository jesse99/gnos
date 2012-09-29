/// Functions and types used to manage a task responsible for managing sample data.
use comm::{Chan, Port};
use RingBuffer = ring_buffer::RingBuffer;

enum Msg
{
	AddSample(~str, float, uint),						// sample set name + sample value, number of samples to retain
	GetSamples(~str, Chan<(~RingBuffer, uint)>),	// sample set name + channel which receives a copy of the buffer and num (global) adds
	ExitMsg,
}

// TODO:
// how is elapsed time tracked?
fn manage_samples(port: comm::Port<Msg>)
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
			GetSamples(copy name, ch) =>
			{
				let buffer = sample_sets[@name];
				ch.send((copy *buffer, num_adds));
			}
			ExitMsg =>
			{
				break;
			}
		}
	}
}

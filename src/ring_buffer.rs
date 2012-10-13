/// Fixed size buffer: when it is at capacity pushs drop the oldest element.
struct RingBuffer
{
	priv mut buffer: ~[float],
	priv capacity: uint,			// number of elements the buffer is able to hold (can't guarantee that vec capacity is exactly what we set it to)
	priv mut size: uint,			// number of elements with legit values in the buffer
	priv mut next: uint,			// index at which new elements land
}

fn RingBuffer(capacity: uint) -> RingBuffer
{
	let ring = RingBuffer {buffer: ~[], capacity: capacity, size: 0, next: 0};
	vec::reserve(ring.buffer, capacity);
	ring
}

impl RingBuffer
{
	pure fn len() -> uint
	{
		self.size
	}
	
	pure fn is_empty() -> bool
	{
		self.size == 0
	}
	
	pure fn is_not_empty() -> bool
	{
		self.size != 0
	}
	
	pure fn buffer() -> uint
	{
		self.size
	}
	
	fn clear()
	{
		vec::truncate(self.buffer, 0);
		self.size = 0;
		self.next = 0;
	}
	
	fn push(element: float)
	{
		assert self.capacity > 0;
		
		if self.size < self.capacity
		{
			vec::push(self.buffer, element);
			self.size += 1;
		}
		else
		{
			self.buffer[self.next] = element;
		}
		self.next = (self.next + 1) % self.capacity;
	}
}

impl RingBuffer : ops::Index<uint, float>
{
	pure fn index(&&index: uint) -> float
	{
		assert index < self.size;
		
		if self.size < self.capacity
		{
			self.buffer[index]
		}
		else
		{
			self.buffer[(self.next + index) % self.capacity]
		}
	}
}

impl RingBuffer : BaseIter<float>
{
	pure fn each(blk: fn(v: &float) -> bool)
	{
		let mut i = 0;
		while i < self.size
		{
			if !blk(&self[i])
			{
				break;
			}
			i += 1;
		}
	}
	
	pure fn size_hint() -> option::Option<uint>
	{
		option::Some(self.size)
	}
}

impl RingBuffer : ToStr
{
	fn to_str() -> ~str
	{
		fmt!("size: %?, next: %?, capacity: %?, buffer: %?", self.size, self.next, self.capacity, self.buffer)
	}
}

#[test]
fn test_ring_buffer()
{
	// size 0
	let buffer = RingBuffer(0);
	assert buffer.len() == 0;
	
	// size 1
	let buffer = RingBuffer(1);
	assert buffer.len() == 0;
	
	buffer.push(2.0);
	assert buffer.len() == 1;
	assert buffer[0] == 2.0;
	
	buffer.push(3.0);
	assert buffer.len() == 1;
	assert buffer[0] == 3.0;
	
	// size 4
	let buffer = RingBuffer(4);
	assert buffer.len() == 0;
	
	buffer.push(1.0);
	assert buffer.len() == 1;
	assert buffer[0] == 1.0;
	
	buffer.push(2.0);
	assert buffer.len() == 2;
	assert buffer[0] == 1.0;
	assert buffer[1] == 2.0;
	
	buffer.push(3.0);
	assert buffer.len() == 3;
	assert buffer[0] == 1.0;
	assert buffer[1] == 2.0;
	assert buffer[2] == 3.0;
	
	buffer.push(4.0);
	assert buffer.len() == 4;
	assert buffer[0] == 1.0;
	assert buffer[1] == 2.0;
	assert buffer[2] == 3.0;
	assert buffer[3] == 4.0;
	
	buffer.push(5.0);
	assert buffer.len() == 4;
	assert buffer[0] == 2.0;
	assert buffer[1] == 3.0;
	assert buffer[2] == 4.0;
	assert buffer[3] == 5.0;
	
	// clear
	buffer.clear();
	assert buffer.len() == 0;
	
	buffer.push(2.0);
	assert buffer.len() == 1;
	assert buffer[0] == 2.0;
	
	buffer.push(3.0);
	assert buffer.len() == 2;
	assert buffer[0] == 2.0;
	assert buffer[1] == 3.0;
}

"use strict";

GNOS.scene = new Scene();
GNOS.timer_id = undefined;
GNOS.last_update = undefined;
GNOS.poll_interval = undefined;

// Thresholds for different meter levels.
GNOS.good_level		= 0.0;
GNOS.ok_level		= 0.5;
GNOS.warn_level		= 0.7;
GNOS.danger_level	= 0.8;

window.onload = function()
{
	resize_canvas();
	window.onresize = resize_canvas;
	
	register_renderer("default map", ["device_info", "device_labels", "device_meters", "poll_interval"], "map", map_renderer);
	register_map_query();
	GNOS.timer_id = setInterval(update_time, 1000);
}

function resize_canvas()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	size_to_window(context);
	if (GNOS.sse_model)
	{
		map_renderer(map, GNOS.sse_model, []);
	}
}

function register_map_query()
{
	// It's rather awkward to have all these OPTIONAL clauses, but according
	// to the spec the entire OPTIONAL block must match in order to affect 
	// the solution.
	var queries = ['											\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?center_x ?center_y ?primary_label ?secondary_label	\
	?tertiary_label ?style ?name								\
WHERE 														\
{																\
	?name gnos:center_x ?center_x .							\
	?name gnos:center_y ?center_y .							\
	OPTIONAL												\
	{															\
		?name gnos:style ?style .								\
	}															\
	OPTIONAL												\
	{															\
		?name gnos:primary_label ?primary_label .			\
	}															\
	OPTIONAL												\
	{															\
		?name gnos:secondary_label ?secondary_label .		\
	}															\
	OPTIONAL												\
	{															\
		?name gnos:tertiary_label ?tertiary_label .				\
	}															\
}',
	'															\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?label ?device ?level ?description							\
WHERE 														\
{																\
	?indicator gnos:meter ?label .								\
	?indicator gnos:target ?device .							\
	?indicator gnos:level ?level .								\
	OPTIONAL												\
	{															\
		?indicator gnos:description ?description .				\
	}															\
}',
	'															\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?poll_interval ?last_update								\
WHERE 														\
{																\
	gnos:map gnos:poll_interval ?poll_interval .				\
	OPTIONAL												\
	{															\
		gnos:map gnos:last_update ?last_update .				\
	}															\
}'];
	register_query("map query", ["device_info", "device_labels", "device_meters", "poll_interval"], "primary", queries, [device_shapes_query, device_meters_query, poll_interval_query]);
}

// solution rows have 
// required fields: name, center_x, center_y
// optional fields: style, primary_label, secondary_label, tertiary_label
function device_shapes_query(solution)
{
	function add_device_label(context, shapes, text, base_styles, style_names)
	{
		var lines = text.split('\n');
		for (var i = 0; i < lines.length; ++i)
		{
			shapes.push(new TextLinesShape(context, Point.zero, [lines[i]], base_styles, style_names));
		}
	}
	
	var infos = [];
	var labels = {};
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	for (var i = 0; i < solution.length; ++i)
	{
		var device = solution[i];
		
		var base_styles = ['identity'];
		if ('style' in device)
			base_styles = device.style.split(' ');
		
		// Record some information for each device.
		infos.push({name: device.name, center_x: device.center_x, center_y: device.center_y, base_styles: base_styles});
		
		// Create shapes for device labels.
		var device_labels = [];
		var label_styles = base_styles.concat('label');
		if ('primary_label' in device)
		{
			add_device_label(context, device_labels, device.primary_label, label_styles, ['primary_label']);
		}
		if ('secondary_label' in device)
		{
			add_device_label(context, device_labels, device.secondary_label, label_styles, ['secondary_label']);
		}
		if ('tertiary_label' in device)
		{
			add_device_label(context, device_labels, device.tertiary_label, label_styles, ['tertiary_label']);
		}
		labels[device.name] = device_labels;
	}
	
	return {device_info: infos, device_labels: labels}
}

// solution rows have 
// required fields: label, device, level
// optional fields: description
function device_meters_query(solution)
{
	var meters = {};
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	for (var i = 0; i < solution.length; ++i)
	{
		var meter = solution[i];
		
		if (meter.level >= GNOS.ok_level)			// TODO: may want an inspector option to show all meters
		{
			if (meter.level < GNOS.ok_level)
				var bar_styles = ['good_level'];
			else if (meter.level < GNOS.warn_level)
				var bar_styles = ['ok_level'];
			else if (meter.level < GNOS.danger_level)
				var bar_styles = ['warn_level'];
			else 
				var bar_styles = ['danger_level'];
				
			var label = "{0}% {1}".format(Math.round(100*meter.level), meter.label);	// TODO: option to show description?
			var label_styles = ['label', 'secondary_label'];
			
			var shape = new ProgressBarShape(context, Point.zero, meter.level, bar_styles, label, label_styles);
			if (!meters[meter.device])
				meters[meter.device] = [];
			meters[meter.device].push(shape);		// note that a device can have multiple meters
		}
	}
	
	return {device_meters: meters}
}

// solution rows have 
// required fields: poll_interval
// optional fields: last_update
function poll_interval_query(solution)
{
	assert(solution.length == 1, "expected one row but found " + solution.length);
	
	var row = solution[0];
	GNOS.last_update = new Date().getTime();
	GNOS.poll_interval = row.poll_interval;
	
	var shape = create_poll_interval_label(GNOS.last_update, GNOS.poll_interval);
	
	return {poll_interval: shape}
}

function update_time()
{
	if (GNOS.scene.shapes.length > 0 && GNOS.poll_interval)
	{
		var shape = create_poll_interval_label(GNOS.last_update, GNOS.poll_interval);
		GNOS.scene.shapes[GNOS.scene.shapes.length-1] = shape;
		
		var map = document.getElementById('map');
		var context = map.getContext('2d');
		context.clearRect(0, 0, map.width, map.height);
		GNOS.scene.draw(context);
	}
}

function create_poll_interval_label(last, poll_interval)
{
	function get_updated_label(last, poll_interval)
	{
		var current = new Date().getTime();
		var last_delta = interval_to_time(current - last);
		
		if (poll_interval)
		{
			var next = last + 1000*poll_interval;
			if (current <= next)
			{
				var next_delta = interval_to_time(next - current);	
				var label = "updated {0} ago (next due in {1})".format(last_delta, next_delta);
				var style_name = "label";
			}
			else if (current < next + 60*1000)		// next will be when modeler starts grabbing new data so there will be a bit of a delay before it makes it all the way to the client
			{
				var label = "updated {0} ago (next is due)".format(last_delta);
				var style_name = "label";
			}
			else
			{
				var next_delta = interval_to_time(current - next);	
				var label = "updated {0} ago (next was due {1} ago)".format(last_delta, next_delta);
				var style_name = "error_label";
			}
		}
		else
		{
			// No longer updating (server has gone down or lost connection).
			var label = "updated {0} ago (not connected)".format(last_delta);
			var style_name = "error_label";
		}
		
		return [label, style_name];
	}
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	var labels = get_updated_label(last, poll_interval);
	var shape = new TextLinesShape(context,
		function (self)
		{
			return new Point(context.canvas.width/2, self.stats.total_height/2);
		}, [labels[0]], ['xsmaller'], [labels[1]]);
	
	return shape;
}

function map_renderer(element, model, model_names)
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	GNOS.scene.remove_all();
	model.device_info.forEach(
		function (device)
		{
			var child_shapes = [];
			child_shapes.push_all(model.device_labels[device.name]);
			child_shapes.push_all(model.device_meters[device.name]);
			// error counts
			
			// Unfortunately we can't create this shape until after all the other sub-shapes are created.
			// So it's simplest just to create the shape here.
			var center = new Point(device.center_x * context.canvas.width, device.center_y * context.canvas.height);
			var shape = new DeviceShape(context, device.name, center, device.base_styles, child_shapes);
			GNOS.scene.append(shape);
		});
	if (model.poll_interval)								// this must be the last shape (we dynamically swap new shapes in)
		GNOS.scene.append(model.poll_interval);
	else
		GNOS.scene.append(new NoOpShape());
	GNOS.scene.draw(context);
}

// ---- DeviceShape class -------------------------------------------------------
// Used to draw a device consisting of a DiscShape and an array of arbitrary shapes.
function DeviceShape(context, name, center, base_styles, shapes)
{
	var width = shapes.reduce(function(value, shape)
	{
		return Math.max(value, shape.width);
	}, 0);
	this.total_height = shapes.reduce(function(value, shape)
	{
		return value + shape.height;
	}, 0);
	var radius = 1.3 * Math.max(this.total_height, width)/2;
	assert(radius > 0.0, "{0} radius is {1}".format(name, radius));
	
	this.disc = new DiscShape(context, new Disc(center, radius), base_styles);
	this.shapes = shapes;
	this.name = name;
	this.clickable = true;
	freezeProps(this);
}

DeviceShape.prototype.draw = function (context)
{
	if (GNOS.selection_name == this.name)
		this.disc.extra_styles = ['selection'];
	else
		this.disc.extra_styles = [];
	this.disc.draw(context);
	
	var dx = this.disc.geometry.center.x;
	var dy = this.disc.geometry.center.y - this.total_height/2;
	for (var i = 0; i < this.shapes.length; ++i)
	{
		context.save();
		
		var shape = this.shapes[i];
		context.translate(dx, dy + shape.height/2);
		
		shape.draw(context);
		
		dy += shape.height;
		context.restore();
	}
}

DeviceShape.prototype.hit_test = function (pt)
{
	return this.disc.hit_test(pt);
}

DeviceShape.prototype.toString = function ()
{
	return "DeviceShape for " + this.name;
}

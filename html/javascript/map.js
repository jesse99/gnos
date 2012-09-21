"use strict";

GNOS.scene = new Scene();

// Thresholds for different meter levels.
GNOS.good_level		= 0.0;
GNOS.ok_level		= 0.5;
GNOS.warn_level		= 0.7;
GNOS.danger_level	= 0.8;

window.onload = function()
{
	resize_canvas();
	window.onresize = resize_canvas;
	
	register_renderer("default map", ["device_info", "device_labels"], "map", map_renderer);
	register_map_query();
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
}'];
	register_query("map query", ["device_info", "device_labels"], "primary", queries, [device_shapes_query]);
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
console.log("   adding label " + [lines[i]]);
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
console.log("found {0} at ({1}, {2})".format(device.name, device.center_x, device.center_y));
		
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
			// meter indicators
			// error counts
			
			// Unfortunately we can't create this shape until after all the other sub-shapes are created.
			// So it's simplest just to create the shape here.
			var center = new Point(device.center_x * context.canvas.width, device.center_y * context.canvas.height);
			var shape = new DeviceShape(context, device.name, center, device.base_styles, child_shapes);
			GNOS.scene.append(shape);
		});
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

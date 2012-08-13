"use strict";

GNOS.scene = new Scene();
GNOS.primary_data = null;
GNOS.alert_data = null;

// Thresholds for different meter levels.
GNOS.good_level		= 0.0;
GNOS.ok_level		= 0.5;
GNOS.warn_level		= 0.7;
GNOS.danger_level	= 0.8;

window.onload = function()
{
	resize_canvas();
	window.onresize = resize_canvas;
	
	var map = document.getElementById('map');
	map.addEventListener("click", handle_canvas_click);
	
	draw_initial_map();
	register_primary_query();
	register_alerts_query();
}

function resize_canvas()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	size_to_window(context);
	if (GNOS.primary_data)
	{
		redraw();
	}
	else
	{
		draw_initial_map();
	}
}

function handle_canvas_click(event)
{
	if (event.button == 0)
	{
		var pos = findPos(this);
		var pt = new Point(event.clientX - pos[0], event.clientY - pos[1]);
		
		var shape = GNOS.scene.hit_test(pt);
		if (shape)
		{
			console.log("clicked {0}".format(shape));
		}
		
		event.preventDefault();
	}
}

function register_primary_query()
{
	// It's rather awkward to have all these OPTIONAL clauses, but according
	// to the spec the entire OPTIONAL block must match in order to affect 
	// the solution.
	var expr = '												\
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
}';

	var expr2 = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?src ?dst ?primary_label ?secondary_label				\
	?tertiary_label ?type ?style									\
WHERE 														\
{																\
	?rel gnos:src ?src .											\
	?rel gnos:dst ?dst .											\
	?rel gnos:type ?type .										\
	OPTIONAL												\
	{															\
		?rel gnos:style ?style .									\
	}															\
	OPTIONAL												\
	{															\
		?rel gnos:primary_label ?primary_label .				\
	}															\
	OPTIONAL												\
	{															\
		?rel gnos:secondary_label ?secondary_label .			\
	}															\
	OPTIONAL												\
	{															\
		?rel gnos:tertiary_label ?tertiary_label .				\
	}															\
}';

	var expr3 = '												\
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
}';

	var expr4 = '												\
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
}';

	var source = new EventSource('/query?name=primary&expr={0}&expr2={1}&expr3={2}&expr4={3}'.
		format(encodeURIComponent(expr), encodeURIComponent(expr2), encodeURIComponent(expr3), encodeURIComponent(expr4)));
	source.addEventListener('message', function(event)
	{
		GNOS.primary_data = event.data;
		populate_shapes();
		redraw();
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('primary stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase === 2)
		{
			console.log('primary stream closed');
		}
	});
}

function register_alerts_query()
{
	var expr = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?device ?count												\
WHERE 														\
{																\
	?device gnos:num_errors ?count							\
}';

	var source = new EventSource('/query?name=alerts&expr={0}'.
		format(encodeURIComponent(expr)));
	source.addEventListener('message', function(event)
	{
		GNOS.alert_data = {};
		var data = JSON.parse(event.data);
		for (var i = 0; i < data.length; ++i)
		{
			var row = data[i];
			GNOS.alert_data[row.device] = row.count;
			//console.log("row{0}: {1:j}".format(i, row));
		}
		
		if (GNOS.primary_data)
		{
			populate_shapes();
			redraw();
		}
	});
	
	source.addEventListener('open', function(event)
	{
		console.log('alerts stream opened');
	});
	
	source.addEventListener('error', function(event)
	{
		if (event.eventPhase === 2)
		{
			console.log('alerts stream closed');
		}
	});
}

function redraw()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	GNOS.scene.draw(context);
}

function populate_shapes()
{
	GNOS.scene.remove_all();
	
	var data = JSON.parse(GNOS.primary_data);
	add_map_label_shapes(data[3]);
	add_device_shapes(data[0], data[2]);
	add_relation_shapes(data[1]);
}

function add_map_label_shapes(times)
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	var row = times[0];
	var labels = get_updated_label(row.last_update, row.poll_interval);
	add_update_label_shapes(context, labels[0], labels[1]);
	
	if (GNOS.alert_data)
		add_alert_label_shapes(context);
}

function add_update_label_shapes(context, label, style_name)
{
	var shape = new TextLinesShape(context,
		function (self)
		{
			return new Point(context.canvas.width/2, self.stats.total_height/2);
		}, [label], ['label', 'xsmaller'], [style_name]);
	GNOS.scene.append(shape);
}

function add_alert_label_shapes(context)
{
	if ('http://www.gnos.org/2012/schema#map' in GNOS.alert_data)
	{
		var count = GNOS.alert_data['http://www.gnos.org/2012/schema#map'];
		if (count)
			var label = "1 error alert";
		else
			var label = "{0} error alerts".format(count);
		
		var shape = new TextLinesShape(context,
			function (self)
			{
				return new Point(context.canvas.width/2, context.canvas.height - self.stats.total_height/2);
			}, [label], ['map', 'label'], ['error_label']);
		GNOS.scene.append(shape);
	}
}

function get_updated_label(last_update, poll_interval)
{
	if (!last_update)
	{
		// missing current (will happen if the modeler machine is slow or fails to respond)
		var label = "store has not been updated";
		var style_name = "error_label";
	}
	else
	{
		var last = new Date(last_update).getTime();
		var current = new Date().getTime();
		var next = last + 1000*poll_interval;
		
		var last_delta = interval_to_time(current - last);
		if (current < next)
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
	
	return [label, style_name];
}

// device has
// required fields: name, center_x, center_y
// optional fields: style, primary_label, secondary_label, tertiary_label
function add_device_shapes(devices, meters)
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	for (var i = 0; i < devices.length; ++i)
	{
		var device = devices[i];
		
		var base_styles = ['identity'];
		if ('style' in device)
			base_styles = device.style.split(' ');
		
		// Device should first draw any labels.
		var shapes = [];
		var label_styles = base_styles.concat('label');
		if ('primary_label' in device)
		{
			shapes.push(new TextLinesShape(context, Point.zero, [device.primary_label], label_styles, ['primary_label']));
		}
		if ('secondary_label' in device)
		{
			shapes.push(new TextLinesShape(context, Point.zero, [device.secondary_label], label_styles, ['secondary_label']));
		}
		if ('tertiary_label' in device)
		{
			shapes.push(new TextLinesShape(context, Point.zero, [device.tertiary_label], label_styles, ['tertiary_label']));
		}
		
		// Then meter indicators.
		for (var j = 0; j < meters.length; ++j)
		{
			var meter = meters[j];
			if (meter.device === device.name && meter.level >= GNOS.ok_level)		// TODO: may want an inspector option to show all meters
			{
				if (meter.level < GNOS.ok_level)
					var bar_styles = base_styles.concat('good_level');
				else if (meter.level < GNOS.warn_level)
					var bar_styles = base_styles.concat('ok_level');
				else if (meter.level < GNOS.danger_level)
					var bar_styles = base_styles.concat('warn_level');
				else 
					var bar_styles = base_styles.concat('danger_level');
					
				var label = "{0}% {1}".format(Math.round(100*meter.level), meter.label);	// TODO: option to show description?
				var label_styles = base_styles.concat(['label', 'secondary_label']);
				
				shapes.push(new ProgressBarShape(context, Point.zero, meter.level, bar_styles, label, label_styles));
			}
		}
		
		// Then error alerts.
		if (GNOS.alert_data && device.name in GNOS.alert_data)
		{
			if (GNOS.alert_data[device.name] === 1)
				var label = "1 error alert";
			else
				var label = "{0} error alerts".format(GNOS.alert_data[device.name]);
			shapes.push(new TextLinesShape(context, Point.zero, [label], label_styles, ['error_label']));
		}
		
		// And, under the foregoing, a disc representing the device.
		var center = new Point(device.center_x * context.canvas.width, device.center_y * context.canvas.height);
		var shape = new DeviceShape(context, device.name, center, base_styles, shapes);
		GNOS.scene.append(shape);
		//console.log("added {0} = {1}".format(device.name, shape));
	}
}

// relation has
//     required fields: src, dst, type
//     optional fields: style, primary_label, secondary_label, tertiary_label
function add_relation_shapes(relations)
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	var infos = find_line_infos(relations);
	var lines = [];
	for (var key in infos)
	{
		var info = infos[key];
		lines.push(add_relation_line_shape(info));
	}
	
	var i = 0;
	for (var key in infos)		// do this after drawing lines so that the labels appear on top
	{
		add_relation_label_shape(context, infos[key].r, lines[i], 0.3);
		if (infos[key].s)
			add_relation_label_shape(context, infos[key].s, lines[i], 0.7);
		i += 1;
	}
}

function add_relation_line_shape(info)
{
	if ('style' in info.r)
		var style = info.r.style;
	else
		var style = 'identity';
		
	if (info.broken)
		var style_names = [style, 'broken_relation'];
	else
		var style_names = [style];
	
	var src = GNOS.scene.find(function (shape) {return shape.name === info.r.src});
	var dst = GNOS.scene.find(function (shape) {return shape.name === info.r.dst});
	
	var line = discs_to_line(src.disc.geometry, dst.disc.geometry);
	line = line.shrink(src.disc.stroke_width/2, dst.disc.stroke_width/2);	// path strokes are centered on the path
	var shape = new LineShape(line, style_names, info.from_arrow, info.to_arrow);
	
	GNOS.scene.append(shape);
	
	return line;
}

function add_relation_label_shape(context, relation, line, p)
{
	if ('style' in relation)
		var style = relation.style;
	else
		var style = 'identity';
		
	// TODO: Should allow labels to have EOL characters. (We don't want to allow multiple
	// labels in the store because the joins get all whacko).
	var lines = [];
	var style_names = [];
	if ('primary_label' in relation)
	{
		lines.push(relation.primary_label);
		style_names.push('primary_relation');
	}
	if ('secondary_label' in relation)
	{
		lines.push(relation.secondary_label);
		style_names.push('secondary_relation');
	}
	if ('tertiary_label' in relation)
	{
		lines.push(relation.tertiary_label);
		style_names.push('tertiary_relation');
	}
	
	var center = line.interpolate(p);
	var base_styles = [style, 'label', 'relation_label'];
	
	var shape = new TextLinesShape(context, center, lines, base_styles, style_names);
	GNOS.scene.append(shape);
}

// Returns object mapping src/dst device subjects to objects of the form:
//     {r: relation, broken: bool, from_arrow: arrow, to_arrow}
function find_line_infos(relations)
{
	var lines = {};
	
	var has_arrow = {stem_height: 16, base_width: 12};
	var no_arrow = {stem_height: 0, base_width: 0};
	
	for (var i=0; i < relations.length; ++i)
	{
		var relation = relations[i];
		
		var key = relation.src < relation.dst ? relation.src + "/" + relation.dst : relation.dst + "/" + relation.src;
		if (relation.type === "undirected")
		{
			// undirected: no arrows
			if (key in lines)
				lines[key] = {r: lines[key].r, s: relation, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
			else
				lines[key] = {r: relation, s: null, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
		}
		else if (relation.type === "unidirectional")
		{
			// unidirectional: arrow for each relation
			if (key in lines)
				lines[key] = {r: lines[key].r, s: relation, broken: false, from_arrow: has_arrow, to_arrow: has_arrow};
			else
				lines[key] = {r: relation, s: null, broken: false, from_arrow: no_arrow, to_arrow: has_arrow};
		}
		else if (relation.type === "bidirectional")
		{
			// two-way bidirectional: no arrows
			// one-way bidirectional: broken (red) arrow
			if (key in lines)
				lines[key] = {r: lines[key].r, s: relation, broken: false, from_arrow: no_arrow, to_arrow: no_arrow};
			else
				lines[key] = {r: relation, s: null, broken: true, from_arrow: no_arrow, to_arrow: has_arrow};
		}
		else
		{
			console.log("Bad relation type: " + relation.type);
		}
	}
	
	return lines;
}

function draw_initial_map()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	var base_styles = ['primary_label', 'xlarger'];
	var lines = ['Loading...'];
	var style_names = ['primary_label'];
	
	var shape = new TextLinesShape(context, new Point(context, map.width/2, map.height/2), lines, base_styles, style_names);
	shape.draw(context);
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
	var radius = 1.1 * Math.max(this.total_height, width)/2;
	
	this.disc = new DiscShape(context, new Disc(center, radius), base_styles);
	this.shapes = shapes;
	this.name = name;
	this.clickable = true;
	freezeProps(this);
}

DeviceShape.prototype.draw = function (context)
{
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

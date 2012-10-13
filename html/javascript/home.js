// Page that shows a map of the network along with details for the map/selection.
"use strict";

GNOS.scene = new Scene();
GNOS.timer_id = undefined;
GNOS.last_update = undefined;
GNOS.poll_interval = undefined;
GNOS.selection_name = null;
GNOS.opened = {};
GNOS.loaded_devices = false;

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
	
	var model_names = ["device_info", "device_labels", "device_meters", "poll_interval", "map_alert_label", "device_alert_labels", "device_relation_infos"];
	register_renderer("map renderer", model_names, "map", map_renderer);
	register_renderer("details renderer", ["selection_details", "selection_alerts"], "details", details_renderer);
	
	register_primary_map_query();
	register_alert_count_query();
	GNOS.timer_id = setInterval(update_time, 1000);
	
	register_selection_query("gnos:map");
	GNOS.selection_name = "gnos:map";
	
	set_loading_label();
};

function resize_canvas()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	size_to_window(context);
	if (!GNOS.loaded_devices)
		set_loading_label();
	map_renderer(map, GNOS.sse_model, []);
}

function set_loading_label()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	var base_styles = ['primary_label', 'xlarger'];
	var lines = ['Loading...'];
	var style_names = ['primary_label'];
	
	var shape = new TextLinesShape(context, new Point(map.width/2, map.height/2), lines, base_styles, style_names);
	GNOS.scene.remove_all();
	GNOS.scene.append(shape);
}

function handle_canvas_click(event)
{
	if (event.button === 0)
	{
		var pos = findPosRelativeToViewport(this);
		var pt = new Point(event.clientX - pos[0], event.clientY - pos[1]);
		
		var shape = GNOS.scene.hit_test(pt);
		if (shape)
			var name = shape.name;
		else
			var name = "gnos:map";
		
		if (name != GNOS.selection_name)
		{
			deregister_selection_query();
			register_selection_query(name);
			GNOS.selection_name = name;
		}
		
		event.preventDefault();
	}
}

function register_selection_query(name)
{
	var query = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?title ?detail ?open	?weight ?key							\
WHERE 														\
{																\
	?details gnos:title ?title .									\
	?details gnos:target <{0}> .								\
	?details gnos:detail ?detail .								\
	?details gnos:weight ?weight .								\
	?details gnos:open ?open .									\
	?details gnos:key ?key 									\
}  ORDER BY DESC(?weight) ASC(?title)'.format(name);
	register_query("selection query", ["selection_details"], "primary", [query], [device_selection_query]);
	
	var oldest = new Date();
	oldest.setDate(oldest.getDate() - 7);	// show alerts for the last week
	if (name === "gnos:map")
		var query = '											\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?target ?mesg ?resolution ?level ?begin ?end				\
WHERE 														\
{																\
	?subject gnos:target ?target .								\
	?subject gnos:mesg ?mesg .								\
	?subject gnos:level ?level .								\
	?subject gnos:begin ?begin .								\
	?subject gnos:resolution ?resolution .						\
	OPTIONAL												\
	{															\
		?subject gnos:end ?end								\
	}															\
	FILTER (?begin >= "{0}"^^xsd:dateTime)				\
} ORDER BY ?begin ?mesg'.format(oldest.toISOString());
	else
		var query = '											\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?mesg ?resolution ?level ?begin ?end						\
WHERE 														\
{																\
	?subject gnos:target <{0}> .								\
	?subject gnos:mesg ?mesg .								\
	?subject gnos:level ?level .								\
	?subject gnos:begin ?begin .								\
	?subject gnos:resolution ?resolution .						\
	OPTIONAL												\
	{															\
		?subject gnos:end ?end								\
	}															\
	FILTER (?begin >= "{1}"^^xsd:dateTime)				\
} ORDER BY ?begin ?mesg'.format(name, oldest.toISOString());
	
	register_query("selection alerts query", ["selection_alerts"], "alerts", [query], [selection_alerts_query]);
}

function deregister_selection_query()
{
	deregister_query("selection query");
	deregister_query("selection alerts query");
	
	GNOS.selection_name = null;
	GNOS.selection_source = null;
}

// solution rows have 
// required fields: title, detail, open, weight, key
function device_selection_query(solution)
{
	function details_to_html(details)
	{
		if (details.open === "always")
		{
			var html = details.detail;
		}
		else
		{
			var open = GNOS.opened[details.key] || details.open === "yes";
			GNOS.opened[details.key] = open;
			
			var handler = "GNOS.opened['{0}'] = !GNOS.opened['{0}']".format(details.key);
			if (open)
				var html = '<details open="open" onclick = "{0}">\n'.format(handler);
			else
				var html = '<details onclick = "{0}">\n'.format(handler);
				
			if (details.title)
				html += '<summary>{0}</summary>\n'.format(details.title);
				
			html += '{0}\n'.format(details.detail);
			html += '</details>\n';
		}
		
		return html;
	}
	
	var details = [];
	for (var i = 0; i < solution.length; ++i)
	{
		var row = solution[i];
		
		var content = details_to_html(row);
		details.push(content);
	}
	
	return {selection_details: details};
}

// solution rows have 
// required fields: mesg, resolution, level, begin
// optional fields: end, target
function selection_alerts_query(solution)
{
	function add_alert(row, options)
	{
		var html = "";
		if (options.levels.indexOf(row.level) >= 0 && (options.kind === "inactive") === 'end' in row)
		{
			if ('end' in row)
				var date = new Date(row.end);
			else
				var date = new Date(row.begin);
				
			if ('target' in row)
			{
				var i = row.target.lastIndexOf('#');
				if (i < 0)
					i = row.target.lastIndexOf('/');
					
				if (i >= 0)
					var target = "{0}: ".format(row.target.slice(i+1));
				else
					var target = "{0}: ".format(row.target);
			}
			else
				var target = "";
				
			var lines = row.mesg.split("\n");
			for (var i = 0; i < lines.length; ++i)
			{
				var attributes = '';
				var classes = '{0}-alert'.format(row.level);
				if (i === 0)
				{
					var targets = escapeHtml(target);
					if (row.resolution)
					{
						classes += ' tooltip';
						attributes += ' data-tooltip=" {0}"'.format(escapeHtml(row.resolution));
					}
					var dates = " ({0})".format(dateToStr(date));
				}
				else
				{
					var targets = "";
					classes += ' indent';
					var dates = "";
				}
				
				html += '<li class="{0}"{1}">{2}{3}{4}</li>\n'.format(
					classes, attributes, targets, escapeHtml(lines[i]), dates);
			}
		}
		return html;
	}
	
	function add_widget(inner, title, open)
	{
		var html = "";
		
		if (inner)
		{
			if (open)
				html += '<details open="open">\n';
			else
				html += '<details>\n';
			html += '	<summary>{0}</summary>\n'.format(title);
			html += "		<ul class='sequence'>\n";
			html += inner;
			html += "		</ul>\n";
			html += '</details>\n';
		}
		
		return html;
	}
	
	var error_alerts = "";
	var warning_alerts = "";
	var info_alerts = "";
	var closed_alerts = "";
	
	for (var i = 0; i < solution.length; ++i)
	{
		var row = solution[i];
		
		error_alerts      += add_alert(row, {levels: ["error"], kind: "active"});
		warning_alerts += add_alert(row, {levels: ["warning"], kind: "active"});
		info_alerts += add_alert(row, {levels: ["info"], kind: "active"});
		closed_alerts    += add_alert(row, {levels: ["error", "warning"], kind: "inactive"});
	}
	
	var html = "";
	html += add_widget(error_alerts, "Error Alerts", true);
	html += add_widget(warning_alerts, "Warning Alerts", false);
	html += add_widget(info_alerts, "Info Alerts", false);
	html += add_widget(closed_alerts, "Closed Alerts", false);
	
	return {selection_alerts: html};
}

function details_renderer(element, model, model_names)
{
	var html = '';
	
	if (model.selection_alerts)
	{
		html += model.selection_alerts + '\n';
	}
	
	if (model.selection_details)
	{
		for (var i = 0; i < model.selection_details.length; ++i)
		{
			var details = model.selection_details[i];
			html += details + '\n';
		}
	}
	
	element.innerHTML = html;
}

function register_primary_map_query()
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
	var model_names = ["device_info", "device_labels", "device_meters", "poll_interval", "device_relation_infos"];
	var callbacks = [device_shapes_query, relations_query, device_meters_query, poll_interval_query];
	register_query("primary map query", model_names, "primary", queries, callbacks);
}

function register_alert_count_query()
{
	var query = '												\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?device ?count												\
WHERE 														\
{																\
	?device gnos:num_errors ?count							\
}';
	register_query("alert count query", ["map_alert_label", "device_alert_labels"], "alerts", [query], [alert_count_query]);
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
	
	if (solution.length > 0)
		GNOS.loaded_devices = true;
	
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
	
	return {device_info: infos, device_labels: labels};
}

// solution rows have 
// required fields: src, dst, type
// optional fields: style, primary_label, secondary_label, tertiary_label
function relations_query(solution)
{
	var lines = {};
	
	var has_arrow = {stem_height: 16, base_width: 12};
	var no_arrow = {stem_height: 0, base_width: 0};
	
	for (var i = 0; i < solution.length; ++i)
	{
		var relation = solution[i];
		
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
	
	// Returns object mapping src/dst device subjects to objects of the form:
	//     {r: relation, broken: bool, from_arrow: arrow, to_arrow}
	return {device_relation_infos: lines};
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
	
	return {device_meters: meters};
}

// solution rows have 
// required fields: poll_interval
// optional fields: last_update
function poll_interval_query(solution)
{
	assert(solution.length <= 1, "expected one row but found " + solution.length);
	
	if (solution.length == 1)
	{
		var row = solution[0];
		GNOS.last_update = new Date().getTime();
		GNOS.poll_interval = row.poll_interval;
		
		var shape = create_poll_interval_label(GNOS.last_update, GNOS.poll_interval);
		
		return {poll_interval: shape};
	}
}

// solution rows have 
// required fields: device, count
function alert_count_query(solution)
{
	var map_label = undefined;
	var device_labels = {};
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	for (var i = 0; i < solution.length; ++i)
	{
		var alert = solution[i];
		
		var label = get_error_alert_count_label(alert.count);
		if (alert.device === "http://www.gnos.org/2012/schema#map")
		{
			map_label = new TextLinesShape(context,
				function (self)
				{
					return new Point(context.canvas.width/2, context.canvas.height - self.stats.total_height/2);
				}, [label], ['map', 'label'], ['error_label']);
		}
		else
		{
			var shape = new TextLinesShape(context, Point.zero, [label], ['label'], ['error_label']);
			if (!device_labels[alert.device])
				device_labels[alert.device] = [];
			device_labels[alert.device].push(shape);
		}
	}
	
	return {map_alert_label: map_label, device_alert_labels: device_labels};
}

function get_error_alert_count_label(count)
{
	if (count === "1")
		return "1 error alert";
	else if (count !== "0")
		return "{0} error alerts".format(count);
	else
		return "";
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

function add_relations_shapes(context, infos)
{
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
		
		var src = GNOS.scene.find(function (shape) {return shape.name === info.r.src;});
		var dst = GNOS.scene.find(function (shape) {return shape.name === info.r.dst;});
		
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

function map_renderer(element, model, model_names)
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	if (GNOS.loaded_devices)
	{
		GNOS.scene.remove_all();
		model.device_info.forEach(
			function (device)
			{
				function push_shape(shapes, model, name)
				{
					if (model && name in model)
						model[name].forEach(function (shape) {shapes.push(shape);});
				}
				
				var child_shapes = [];
				push_shape(child_shapes, model.device_labels, device.name);
				push_shape(child_shapes, model.device_meters, device.name);
				push_shape(child_shapes, model.device_alert_labels, device.name);
				
				// Unfortunately we can't create this shape until after all the other sub-shapes are created.
				// So it's simplest just to create the shape here.
				var center = new Point(device.center_x * context.canvas.width, device.center_y * context.canvas.height);
				var shape = new DeviceShape(context, device.name, center, device.base_styles, child_shapes);
				GNOS.scene.append(shape);
			});
		if (model.device_relation_infos)
			add_relations_shapes(context, model.device_relation_infos);
			
		if (model.map_alert_label)
			GNOS.scene.append(model.map_alert_label);
			
		if (model.poll_interval)								// this must be the last shape (we dynamically swap new shapes in)
			GNOS.scene.append(model.poll_interval);
		else
			GNOS.scene.append(new NoOpShape());
	}
	
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
};

DeviceShape.prototype.hit_test = function (pt)
{
	return this.disc.hit_test(pt);
};

DeviceShape.prototype.toString = function ()
{
	return "DeviceShape for " + this.name;
};

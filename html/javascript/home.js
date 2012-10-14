// Page that shows a map of entities and the relations between them
"use strict";

GNOS.scene = new Scene();
//GNOS.timer_id = undefined;
//GNOS.last_update = undefined;
//GNOS.poll_interval = undefined;
//GNOS.selection_name = null;
//GNOS.opened = {};
GNOS.entity_detail= undefined;
//GNOS.relation_detail= undefined;
GNOS.loaded_entities = false;

window.onload = function()
{
	resize_canvas();
	window.onresize = resize_canvas;
	
	GNOS.entity_detail = document.getElementById('entity_detail');

	var model_names = ["entities", "labels"];
	GNOS.entity_detail.onchange = function () {do_model_changed(model_names, false);};
	register_renderer("map renderer", model_names, "map", map_renderer);
	
	register_primary_map_query();
//	GNOS.timer_id = setInterval(update_time, 1000);
	
	set_loading_label();
};

function resize_canvas()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	size_to_window(context);
	if (!GNOS.loaded_entities)
		set_loading_label();
	map_renderer(map, GNOS.sse_model, []);
}

function set_loading_label()
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	
	var shape = new TextLineShape(context, new Point(map.width/2, map.height/2), 'Loading...', ["font-size:xx-large", "font-size:larger", "font-size:larger"]);
	GNOS.scene.remove_all();
	GNOS.scene.append(shape);
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
	?target ?title ?style ?predicate								\
WHERE 														\
{																\
	?target gnos:entity ?title .									\
	OPTIONAL												\
	{															\
		?target gnos:style ?style		 .						\
	}															\
	OPTIONAL												\
	{															\
		?target gnos:predicate ?predicate .						\
	}															\
}',
	'															\
PREFIX gnos: <http://www.gnos.org/2012/schema#>		\
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>	\
SELECT 														\
	?label ?target ?level ?style	?predicate						\
WHERE 														\
{																\
	?info gnos:label ?label .									\
	?info gnos:target ?target .									\
	?info gnos:level ?level .									\
	OPTIONAL												\
	{															\
		?info gnos:style ?style .								\
	}															\
	OPTIONAL												\
	{															\
		?info gnos:predicate ?predicate .						\
	}															\
}'];
	var model_names = ["entities", "labels"];
	var callbacks = [entities_query, labels_query];
	register_query("primary map query", model_names, "primary", queries, callbacks);
}

// solution rows have 
// required fields: target, title
// optional fields: style, predicate
function entities_query(solution)
{
	if (solution.length > 0)
		GNOS.loaded_entities = true;
	
	var entities = [];
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	for (var i = 0; i < solution.length; ++i)
	{
		var row = solution[i];
		
		var style = row.style || "";
		var styles = style.split(' ');
		var label = new TextLineShape(context, Point.zero, row.title, styles);	// TODO: only include font styles
		
		entities.push({target: row.target, title: label, styles: styles, predicate: row.predicate || ""});
	}
	
	return {entities: entities};
}

// solution rows have 
// required fields: label, target, level
// optional fields: style, predicate
function labels_query(solution)
{
	var labels = [];
	
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	for (var i = 0; i < solution.length; ++i)
	{
		var row = solution[i];
		
		var style = row.style || "";
		var styles = style.split(' ');
		var label = new TextLineShape(context, Point.zero, row.label, styles);
		
		labels.push({target: row.target, label: label, level: row.level, predicate: row.predicate || ""});
	}
	
	return {labels: labels};
}

function map_renderer(element, model, model_names)
{
	var map = document.getElementById('map');
	var context = map.getContext('2d');
	context.clearRect(0, 0, map.width, map.height);
	
	if (GNOS.loaded_entities)
	{
		var max_entity = 0;
		
		GNOS.scene.remove_all();
		model.entities.forEach(
			function (entity, i)
			{
				var child_shapes = [];
				child_shapes.push(entity.title);
				
				model.labels.forEach(
					function (label)
					{
						if (label.target === entity.target && label.level <= GNOS.entity_detail.value)
						{
							child_shapes.push(label.label);
						}
						
						max_entity = Math.max(label.level, max_entity);
					});
				
				// Unfortunately we can't create this shape until after all the other sub-shapes are created.
				// So it's simplest just to create the shape here.
				var center = new Point(200 + 200*i, 50 + 200*i);
//				var center = new Point(50 + 50*i * context.canvas.width, 50 + 50*i * context.canvas.height);
				var shape = new EntityShape(context, entity.target, center, entity.styles, child_shapes);
				GNOS.scene.append(shape);
			});
			
		// This is the only place where we know all of the levels of the entity infos.
		// If the range has changed we update the slider accordingly. (It's a bit weird
		// that we also use the slider value here but we can't do better).
		GNOS.entity_detail.max = max_entity;
		GNOS.entity_detail.hidden = max_entity === 0;
		
		if (model.poll_interval)								// this must be the last shape (we dynamically swap new shapes in)
			GNOS.scene.append(model.poll_interval);
		else
			GNOS.scene.append(new NoOpShape());
	}
	
	GNOS.scene.draw(context);
}

// ---- EntityShape class -------------------------------------------------------
// Used to draw a device consisting of a RectShape and an array of arbitrary shapes.
function EntityShape(context, name, center, entity_styles, shapes)
{
	var width = 14 + shapes.reduce(function(value, shape)
	{
		return Math.max(value, shape.width);
	}, 0);
	this.total_height = 8 + shapes.reduce(function(value, shape)
	{
		return value + shape.height;
	}, 0);
	
	this.rect = new RectShape(context, new Rect(center.x - width/2, center.y - this.total_height/2, width, this.total_height), entity_styles);
	this.shapes = shapes;
	this.name = name;
	this.clickable = true;
	freezeProps(this);
}

EntityShape.prototype.draw = function (context)
{
//	if (GNOS.selection_name == this.name)
//		this.rect.extra_styles = ['selection'];
//	else
//		this.rect.extra_styles = [];
	this.rect.draw(context);
	
	var dx = this.rect.geometry.left + this.rect.width/2;
	var dy = this.rect.geometry.top + this.rect.height/2 - this.total_height/2 + 0.2*this.shapes[0].height;	// TODO: hopefully when text metrics work a bit better we can get rid of that last term (think we need leading)
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

EntityShape.prototype.hit_test = function (pt)
{
	return this.rect.hit_test(pt);
};

EntityShape.prototype.toString = function ()
{
	return "EntityShape for " + this.name;
};

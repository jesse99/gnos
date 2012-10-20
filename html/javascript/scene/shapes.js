// Immutable objects that scene draws, hit tests, and (eventually) moves.
"use strict";

// ---- NoOpShape class -------------------------------------------------------
function NoOpShape()
{
	this.geometry = Point.zero;
	this.width = 0;
	this.height = 0;
	freezeProps(this);
}

NoOpShape.prototype.draw = function (context)
{
};

NoOpShape.prototype.hit_test = function (pt)
{
	return false;
};

NoOpShape.prototype.toString = function ()
{
	return "NoOpShape";
};

// ---- LineShape class -------------------------------------------------------
// arrows are objects with stem_height and base_width properties
function LineShape(context, line, styles, from_arrow, to_arrow)
{
	this.geometry = line;
	this.styles = ['line-color:black'].concat(styles).filter(function (s) {return s.indexOf('line-') === 0;});
	this.width = Math.abs(this.geometry.from.x - this.geometry.to.x);
	this.height = Math.abs(this.geometry.from.y - this.geometry.to.y);
	
	context.save();
	apply_styles(context, this.styles);
	this.stroke_width = context.lineWidth;
	context.restore();
	
	if (styles.indexOf("line-type:directed") >= 0)
	{
		this.from_arrow = {stem_height: 0, base_width: 0};
		this.to_arrow = {stem_height: 15 + this.stroke_width, base_width: 12 + this.stroke_width};
	}
	else if (styles.indexOf("line-type:bidirectional") >= 0)
	{
		this.from_arrow = {stem_height: 15 + this.stroke_width, base_width: 12 + this.stroke_width};
		this.to_arrow = {stem_height: 15 + this.stroke_width, base_width: 12 + this.stroke_width};
	}
	else
	{
		this.from_arrow = {stem_height: 0, base_width: 0};
		this.to_arrow = {stem_height: 0, base_width: 0};
	}
	
	freezeProps(this);
}

// TODO: scene should take care of saving/restoring context
LineShape.prototype.draw = function (context)
{
	context.save();
	apply_styles(context, this.styles);
	
	var unit = this.geometry.normalize();
	var from_x = this.geometry.from.x + this.from_arrow.stem_height * unit.x;
	var from_y = this.geometry.from.y + this.from_arrow.stem_height * unit.y;
	var to_x = this.geometry.to.x - this.to_arrow.stem_height * unit.x;
	var to_y = this.geometry.to.y - this.to_arrow.stem_height * unit.y;
	
	if (context.frameBlur)
	{
		context.shadowBlur = context.frameBlur;
		context.shadowColor = context.strokeStyle;
		context.shadowOffsetX = context.lineWidth;
		context.shadowOffsetY = context.lineWidth;
	}
	context.beginPath();
		context.moveTo(from_x, from_y);
		context.lineTo(to_x, to_y);
	context.stroke();
	
	if (this.from_arrow.stem_height > 0)
		this.do_draw_arrow(context, unit, this.geometry.from, from_x, from_y, this.from_arrow);
	if (this.to_arrow.stem_height > 0)
		this.do_draw_arrow(context, unit, this.geometry.to, to_x, to_y, this.to_arrow);
		
	context.restore();
};

LineShape.prototype.hit_test = function (pt)
{
	return false;		// TODO: not implemented
};

LineShape.prototype.toString = function ()
{
	return "LineShape at " + this.geometry.toString();
};

LineShape.prototype.do_draw_arrow = function(context, unit, tip, x, y, arrow)
{
	context.fillStyle = context.strokeStyle;
	var normals = this.geometry.normals();
	
	context.beginPath();
	context.moveTo(tip.x, tip.y);
	context.lineTo(x + (arrow.base_width/2) * normals[0].x, y + (arrow.base_width/2) * normals[0].y);
	context.lineTo(x + (arrow.base_width/2) * normals[1].x, y + (arrow.base_width/2) * normals[1].y);
	context.closePath();
	context.fill();
};

// ---- GaugeShape class ------------------------------------------------
// value should be in [0, 1].
function GaugeShape(context, center, value, title, styles, priority)
{
	assert(value >= 0 && value <= 1, "value is oor: " + value);
	this.base_styles = styles;
	this.priority = priority;
	
	this.label = new TextLineShape(context, center, title, ['font-size:small'].concat(styles));
	this.base_width = 1.3*this.label.width;
	this.width = this.base_width;
	this.height = 1.1*this.label.height;
	
	var rect = new Rect(center.x - this.width/2, center.y - this.height/2, this.width, this.height);
	this.frame = new RectShape(context, rect, ['frame-width:1', 'frame-color:silver'].concat(styles));
	
	this.styles = ['gauge-bar-color:lightblue'].concat(styles).filter(function (s) {return s.indexOf('gauge-') === 0;});
	this.geometry = center;
	this.value = value;
}

GaugeShape.prototype.adjust_width = function (context, width)
{
	this.width = Math.max(width, this.base_width);
	var rect = new Rect(this.geometry.x - this.width/2, this.geometry.y - this.height/2, this.width, this.height);
	this.frame = new RectShape(context, rect, ['frame-width:1', 'frame-color:silver'].concat(this.base_styles));
};

GaugeShape.prototype.draw = function (context)
{
	this.frame.draw(context);
	
	context.save();
		var width = this.width - 2*this.frame.stroke_width;
		var height = this.height - 2*this.frame.stroke_width;
		
		apply_styles(context, this.styles);
		context.clearRect(this.geometry.x - width/2, this.geometry.y - height/2, width, height);
		context.fillRect(this.geometry.x - width/2, this.geometry.y - height/2, this.value*width, height);
	context.restore();
	
	this.label.draw(context);
};

GaugeShape.prototype.hit_test = function (pt)
{
	return false;		// TODO: not implemented
};

GaugeShape.prototype.toString = function ()
{
	return "GaugeShape at " + this.geometry.toString();
};

// ---- DiscShape class -------------------------------------------------------
// Draws a filled disc. If context.lineWidth is non-zero a border is also added.
function DiscShape(context, disc, styles)
{
	this.geometry = disc;
	this.styles = ['frame-color:black', 'frame-back-color:white'].concat(styles).filter(function (s) {return s.indexOf('frame-') === 0;});
	this.width = disc.radius;
	this.height = disc.radius;
	
	context.save();
	apply_styles(context, this.styles);
	this.stroke_width = context.lineWidth;
	context.restore();
	
	freezeProps(this);
}

DiscShape.prototype.draw = function (context)
{
	context.save();
	apply_styles(context, this.styles);
	//console.log("drawing disc with {0:j} and {1:j}".format(this.style_names, compose_styles(this.style_names)));
	
	if (context.frameBlur)
	{
		context.shadowBlur = context.frameBlur;
		context.shadowColor = context.strokeStyle;
		context.shadowOffsetX = context.lineWidth;
		context.shadowOffsetY = context.lineWidth;
	}
	
	context.beginPath();
		context.arc(this.geometry.center.x, this.geometry.center.y, this.geometry.radius, 0, 2*Math.PI);
	context.closePath();
	
	context.fill();
	if (context.lineWidth !== 0)
		context.stroke();
	context.restore();
};

DiscShape.prototype.hit_test = function (pt)
{
	var d2 = this.geometry.center.distance_squared(pt);
	var r = this.geometry.radius + this.stroke_width/2;
	return d2 <= r*r;
};

DiscShape.prototype.toString = function ()
{
	return "DiscShape at " + this.geometry.toString();
};

// ---- RectShape class -------------------------------------------------------
// Draws a filled rectangle. If context.lineWidth is non-zero a border is also added.
function RectShape(context, rect, styles)
{
	this.geometry = rect;
	this.styles = ['frame-color:black', 'frame-back-color:white'].concat(styles).filter(function (s) {return s.indexOf('frame-') === 0;});
	this.width = rect.width;
	this.height = rect.height;
	
	context.save();
	apply_styles(context, this.styles);
	this.stroke_width = context.lineWidth;
	context.restore();
	
	freezeProps(this);
}

RectShape.prototype.draw = function (context)
{
	function fill_rect(context, rect)
	{
		context.beginPath();
			context.moveTo(rect.left, rect.top);
			
			// top
			context.lineTo(rect.left + rect.width, rect.top);
			
			// right
			context.lineTo(rect.left + rect.width, rect.top + rect.height);
			
			// bottom
			context.lineTo(rect.left, rect.top + rect.height);
		context.closePath();
		context.fill();
	}
	
	function stroke_left_top(context, rect)
	{
		context.beginPath();
			context.moveTo(rect.left, rect.top + rect.height);
			
			// left
			context.lineTo(rect.left, rect.top);
			
			// top
			context.lineTo(rect.left + rect.width, rect.top);
		context.stroke();
	}
	
	function stroke_right_bottom(context, rect)
	{
		context.beginPath();
			context.moveTo(rect.left + rect.width, rect.top);
			
			// right
			context.lineTo(rect.left + rect.width, rect.top + rect.height);
			
			// bottom
			context.lineTo(rect.left, rect.top + rect.height);
		context.stroke();
	}
	
	context.save();
	apply_styles(context, this.styles);
	fill_rect(context, this.geometry);
	
	if (context.lineWidth !== 0)
	{
		stroke_left_top(context, this.geometry);
		
		if (context.frameBlur)
		{
			context.shadowBlur = context.frameBlur;
			context.shadowColor = context.strokeStyle;
			context.shadowOffsetX = context.lineWidth;
			context.shadowOffsetY = context.lineWidth;
		}
		stroke_right_bottom(context, this.geometry);
	}
	
	context.restore();
};

RectShape.prototype.hit_test = function (pt)
{
	if (pt.x >= this.geometry.left && pt.x < this.geometry.left + this.geometry.width)
	{
		if (pt.y >= this.geometry.top && pt.y < this.geometry.top + this.geometry.height)
		{
			return true;
		}
	}
	return false;
};

// Does an intersection of the perimeter of the shape with a line from center to other (center).
RectShape.prototype.intersect_line = function (other)
{
	var g = this.geometry;
	var line1 = new Line(new Point(g.left + g.width/2, g.top + g.height/2), other);
	
	// try left side
	var result = new Line(new Point(g.left, g.top), new Point(g.left, g.top + g.height)).intersection(line1);
	if (result)
		return result;
		
	// try right side
	result = new Line(new Point(g.left + g.width, g.top), new Point(g.left + g.width, g.top + g.height)).intersection(line1);
	if (result)
		return result;
		
	// try top side
	result = new Line(new Point(g.left, g.top), new Point(g.left + g.width, g.top)).intersection(line1);
	if (result)
		return result;
		
	// try bottom side
	result = new Line(new Point(g.left, g.top + g.height), new Point(g.left + g.width, g.top + g.height)).intersection(line1);
	assert(result, "{0} and {1} did not intersect".format(this, other));
	return result;
};

RectShape.prototype.toString = function ()
{
	return "RectShape at " + this.geometry.toString();
};

// ---- TextLineShape class ----------------------------------------------------
// Draws a line of text. 
// center - Point indicating where the text should be drawn. Currently the text will be centered on this.
//              Or a function taking a this argument returning a Point.
// text - The string to draw.
// style - Array of style name:value
function TextLineShape(context, center, text, styles, priority)
{
	this.geometry = Point.zero;
	this.text = text;
	this.styles = ['font-color:black'].concat(styles).filter(function (s) {return s.indexOf('font-') === 0;});
	this.priority = priority;
	
	this.stats = this.do_prep_center_text(context);
	this.width = this.stats.width;
	this.height = this.stats.height;
	
	if (typeof(center) == "function")
		this.geometry = center(this);
	else
		this.geometry = center;
	this.bbox = new Rect(this.geometry.x - this.width/2, this.geometry.y - this.height/2, this.width, this.height);
	freezeProps(this);
}

TextLineShape.prototype.draw = function (context)
{
	context.save();
	
	if (this.text)
	{
//		if (style['clearRect'])
//			context.clearRect(this.geometry.x - this.stats.max_width/2, this.geometry.y - this.stats.total_height/2, this.stats.max_width, this.stats.total_height);
		
		context.textAlign = 'center';
		context.textBaseline = 'top';
		
		var y = this.geometry.y - this.stats.height/2;
		var style = apply_styles(context, this.styles);
		//console.log("drawing '{0} at {1}pt".format(line, style.fontSize));
		
		context.fillText(this.text, this.geometry.x, y);
	}
	
	// Note that we always need to restore the context to avoid tripping the
	// assert in Scene.prototype.draw.
	context.restore();
};

TextLineShape.prototype.hit_test = function (pt)
{
	return false;		// TODO: not implemented
};

TextLineShape.prototype.toString = function ()
{
	return "TextLineShape at " + this.geometry.toString();
};

// Returns an object with:
//    height: height of the text in pixels
//    width: width of the text in pixels
TextLineShape.prototype.do_prep_center_text = function(context)
{
	var height = 0;
	var width = 0;
	
	if (this.text)
	{
		context.save();
		
		context.textAlign = 'center';
		context.textBaseline = 'top';
		
		var metrics = compute_text_metrics(context, [this.text], [this.styles]);
		height = metrics.heights[0];
		width = metrics.widths[0];
		
		context.restore();
	}
	
	return {height: height, width: width};
};

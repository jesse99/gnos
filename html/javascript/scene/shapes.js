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
function LineShape(line, styles, from_arrow, to_arrow)
{
	this.geometry = line;
	this.styles = ['frame-color:black'].concat(styles);
	this.from_arrow = from_arrow;
	this.to_arrow = to_arrow;
	this.width = Math.abs(this.geometry.from.x - this.geometry.to.x);
	this.height = Math.abs(this.geometry.from.y - this.geometry.to.y);
	freezeProps(this);
}

// TODO: scene should take care of saving/restoring context
LineShape.prototype.draw = function (context)
{
	context.save();
	apply_styles(context, this.style_names);
	
	var unit = this.geometry.normalize();
	var from_x = this.geometry.from.x + this.from_arrow.stem_height * unit.x;
	var from_y = this.geometry.from.y + this.from_arrow.stem_height * unit.y;
	var to_x = this.geometry.to.x - this.to_arrow.stem_height * unit.x;
	var to_y = this.geometry.to.y - this.to_arrow.stem_height * unit.y;
	
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
	context.fill();
};

// ---- ProgressBarShape class ------------------------------------------------
// bar_width should be in [0, 1].
function ProgressBarShape(context, center, bar_width, bar_styles, label, label_styles)
{
	assert(bar_width >= 0 && bar_width <= 1, "bar_width is oor");
	
	this.label = new TextLinesShape(context, center, [label], label_styles.slice(0, label_styles.length-1), label_styles.slice(label_styles.length-1));
	
	this.geometry = center;
	this.bar_styles = bar_styles;
	this.bar_width = bar_width;
	this.width = 1.3*this.label.width;
	this.height = 1.1*this.label.height;
	freezeProps(this);
}

ProgressBarShape.prototype.draw = function (context)
{
	context.save();
	
	apply_styles(context, this.bar_styles);
	context.clearRect(this.geometry.x - this.width/2, this.geometry.y - this.height/2, this.width, this.height);
	context.fillRect(this.geometry.x - this.width/2, this.geometry.y - this.height/2, this.bar_width*this.width, this.height);
	
	context.restore();
	
	this.label.draw(context);
};

ProgressBarShape.prototype.hit_test = function (pt)
{
	return false;		// TODO: not implemented
};

ProgressBarShape.prototype.toString = function ()
{
	return "ProgressBarShape at " + this.geometry.toString();
};

// ---- DiscShape class -------------------------------------------------------
// Draws a filled disc. If context.lineWidth is non-zero a border is also added.
function DiscShape(context, disc, styles)
{
	this.geometry = disc;
	this.styles = ['frame-color:black', 'back-color:white'].concat(styles);
	this.width = disc.radius;
	this.height = disc.radius;
	
	context.save();
	apply_styles(context, this.styles);
	this.stroke_width = context.lineWidth;
	context.restore();
	
	freezeProps(this);
	this.extra_styles = [];
}

DiscShape.prototype.draw = function (context)
{
	context.save();
	
	apply_styles(context, this.styles);
	//console.log("drawing disc with {0:j} and {1:j}".format(this.style_names, compose_styles(this.style_names)));
	
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

// ---- TextLineShape class ----------------------------------------------------
// Draws a line of text. 
// center - Point indicating where the text should be drawn. Currently the text will be centered on this.
//              Or a function taking a this argument returning a Point.
// text - The string to draw.
// style - Array of style name:value
function TextLineShape(context, center, text, styles)
{
	this.geometry = Point.zero;
	this.text = text;
	this.styles = ['font-color:black'].concat(styles);
	
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

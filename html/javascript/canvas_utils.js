"use strict";

// ---- Point class -----------------------------------------------------------
// All coordinates are screen coordinates.
function Point(x, y)
{
	this.x = x;
	this.y = y;
	freezeProps(this);
}

Point.zero = new Point(0, 0);

// Returns the distance between this and rhs.
Point.prototype.distance = function (rhs)
{
	var dx = this.x - rhs.x;
	var dy = this.y - rhs.y;
	return Math.sqrt(dx*dx + dy*dy);
}

Point.prototype.toString = function ()
{
	return "{x: " + this.x + ", y: " + this.y + "}";
}

// ---- Line class ------------------------------------------------------------
function Line(from, to)
{
	this.from = from;
	this.to = to;
	freezeProps(this);
}

Line.prototype.normals = function ()
{
	var unit = this.normalize();
	return [new Point(-unit.y, unit.x), new Point(unit.y, -unit.x)];
}

// Returns the line as a unit vector.
Line.prototype.normalize = function ()
{
	var x = this.to.x - this.from.x;
	var y = this.to.y - this.from.y;
	var d = this.to.distance(this.from);
	return new Point(x/d, y/d);
}

// Returns a point on the line from this.from (0.0) to this.to (1.0).
Line.prototype.interpolate = function (p)
{
	assert(p >= 0.0, "position is negative");
	assert(p <= 1.0, "position is larger than 1.0");
	
	var dx = this.to.x - this.from.x;
	var dy = this.to.y - this.from.y;
	
	return new Point(this.from.x + p*dx, this.from.y + p*dy);
}

// Shrinks the line by from_delta pixels on the from side and to_delta pixels on the to side.
Line.prototype.shrink = function (from_delta, to_delta)
{
	var theta = Math.atan((this.to.y - this.from.y)/(this.to.x - this.from.x));
	
	var dx = from_delta * Math.cos(theta);
	var dy = from_delta * Math.sin(theta);
	var from = new Point(this.from.x + dx, this.from.y + dy);
		
	dx = to_delta * Math.cos(theta);
	dy = to_delta * Math.sin(theta);
	var to = new Point(this.to.x + dx, this.to.y + dy);
	
	return new Line(from, to);
}

Line.prototype.toString = function ()
{
	return "{from: " + this.from + ", to: " + this.to + "}";
}

// ---- Disc class ------------------------------------------------------------
function Disc(center, radius)
{
	this.center = center;
	this.radius = radius;
	freezeProps(this);
}

// Returns true if this intersects the rhs disc.
Disc.prototype.intersects = function (rhs)
{
	return this.center.distance(rhs.center) <= this.radius || rhs.center.distance(this.center) <= rhs.radius;
}

// Returns the point on this perimeter closest to pt.
Disc.prototype.intersection = function (pt)
{
	var dx = pt.x - this.center.x;
	var dy = pt.y - this.center.y;
	var theta = Math.atan(dy/dx);
	
	var x = this.center.x + this.radius * Math.cos(theta);
	var y = this.center.y + this.radius * Math.sin(theta);
	var pt1 = new Point(x, y);
	
	x = this.center.x - this.radius * Math.cos(theta);
	y = this.center.y - this.radius * Math.sin(theta);
	var pt2 = new Point(x, y);
	
	if (pt1.distance(pt) < pt2.distance(pt))
		return pt1;
	else
		return pt2;
}

Disc.prototype.toString = function ()
{
	return "{center: " + this.center + ", radius: " + this.radius + "}";
}

// ---- Drawing Functions -----------------------------------------------------
// from and to are Points.
// to_arrow is an object with stem_height and base_width properties
function draw_line(context, styles, line, from_arrow, to_arrow)
{
	context.save();
	apply_styles(context, styles);
	
	var unit = line.normalize();
	var from_x = line.from.x + from_arrow.stem_height * unit.x;
	var from_y = line.from.y + from_arrow.stem_height * unit.y;
	var to_x = line.to.x - to_arrow.stem_height * unit.x;
	var to_y = line.to.y - to_arrow.stem_height * unit.y;
	
	context.beginPath();
	context.moveTo(from_x, from_y);
	context.lineTo(to_x, to_y);
	context.stroke();
	
	if (from_arrow.stem_height > 0)
		do_draw_arrow(context, line, unit, line.from, from_x, from_y, from_arrow);
	if (to_arrow.stem_height > 0)
		do_draw_arrow(context, line, unit, line.to, to_x, to_y, to_arrow);
		
	context.restore();
}

// Draws a filled disc. If lineWidth is non-zero a border is also added.
// at is a Point.
function draw_disc(context, styles, disc)
{
	context.save();
	var style = apply_styles(context, styles);
	//console.log("drawing disc with {0:j} and {1:j}".format(styles, compose_styles(styles)));
	
	context.beginPath();
	context.arc(disc.center.x, disc.center.y, disc.radius, 0, 2*Math.PI);
	context.closePath();
	
	context.fill();
	if (context.lineWidth != 0)
		context.stroke();
	context.restore();
	
	return style;
}

// Returns an object with:
//    total_height: of all the lines
//    max_width: width of the widest line
// and misc fields for internal use.
function prep_center_text(context, base_styles, lines, styles)
{
	var total_height = 0.0;
	var max_width = 0.0;
	var heights = [];
	
	if (lines)
	{
		context.save();
		
		context.textAlign = 'center';
		context.textBaseline = 'top';
		
		heights = do_compute_line_heights(context, base_styles, styles);
		total_height = heights.reduce(function(previous, current, i, array)
		{
			return previous + current;
		}, 0);
		
		for (var i=0; i < lines.length; ++i)
		{
			var line = lines[i];
			apply_styles(context, base_styles.concat(styles[i]));
			
			var metrics = context.measureText(line);
			max_width = Math.max(metrics.width, max_width);
		}
		
		context.restore();
	}
	
	return {total_height: total_height, max_width: max_width, heights: heights};
}

// Draw lines of text centered on a Point. This is a bit complex because
// each line may be styled differently.
function center_text(context, base_styles, lines, styles, center, stats)
{
	if (lines)
	{
		context.save();
		
		context.textAlign = 'center';
		context.textBaseline = 'top';
		context.fillStyle = 'black';
		
		var y = center.y - stats.total_height/2;
		
		for (var i=0; i < lines.length; ++i)
		{
			var line = lines[i];
			var style = apply_styles(context, base_styles.concat(styles[i]));
			//console.log("drawing '{0} at {1}pt".format(line, style.fontSize));
			
			context.fillText(line, center.x, y);
			y += stats.heights[i];
		}
		
		context.restore();
	}
}

// Returns a Line between the perimeter of the two discs.
function discs_to_line(disc1, disc2)
{
	if (!disc1.intersects(disc2))
	{
		return new Line(disc1.intersection(disc2.center), disc2.intersection(disc1.center));
	}
	else
	{
		return new Line(Point.zero, Point.zero);
	}
}

// ---- Internal Functions ----------------------------------------------------
function do_draw_arrow(context, line, unit, tip, x, y, arrow)
{
	context.fillStyle = context.strokeStyle;
	var normals = line.normals();
	
	context.beginPath();
	context.moveTo(tip.x, tip.y);
	context.lineTo(x + (arrow.base_width/2) * normals[0].x, y + (arrow.base_width/2) * normals[0].y);
	context.lineTo(x + (arrow.base_width/2) * normals[1].x, y + (arrow.base_width/2) * normals[1].y);
	context.fill();
}

function do_compute_line_heights(context, base_styles, styles)
{
	var heights = [];
	context.save();
	
	for (var i=0; i < styles.length; ++i)
	{
		apply_styles(context, base_styles.concat(styles[i]));
		
		var line_height = do_compute_line_height(context);
		heights.push(line_height);
	}
	
	context.restore();
	return heights;
}

function do_compute_line_height(context)
{
	var metrics = context.measureText("W");
	
	// As of July 2012 Chrome only has width inside metrics.
	if ('fontBoundingBoxAscent' in metrics)
		var line_height = metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent;
	else
		var line_height = metrics.width + metrics.width/6;	// this is what Core HTML5 Canvas recommends
		
	return line_height;
}


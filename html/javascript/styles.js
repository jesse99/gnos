"use strict";

// Table mapping names to styles.
GNOS.styles =
{
	'default':
	{
		fontStyle: 'normal',		// or italic, oblique
		fontWeight: 400,		// 100 to 900 where normal is 400 and 700 is bold
		fontSize: 10,				// in points
		fontFamily: 'arial',		// font name (TODO: add web safe fonts) or serif, sans-serif, cursive, monospace
		
		lineWidth: 1,
		strokeStyle: "black",	
		fillStyle: "black",	
	},
	
	'map':						{fontSize: xlarger},
	'host':						{lineWidth: 2, strokeStyle: 'black', fillStyle: 'lightblue', fontSize: smaller},
	'router':					{lineWidth: 8, strokeStyle: 'black', fillStyle: 'mistyrose'},
	'switch':					{lineWidth: 2, strokeStyle: 'black', fillStyle: 'lavender'},
	
	'identity':					{},
	'link':						{},
	'route':					{lineWidth: 4, strokeStyle: 'royalblue'},
	'broken_relation':		{strokeStyle: 'red'},
	
	'relation_label':			{},
	'primary_relation':		{},
	'secondary_relation':	{fontSize: smaller},
	'tertiary_relation':		{fontSize: smaller},
	
	'label':						{strokeStyle: 'black', fillStyle: 'black'},
	'primary_label':			{fontWeight: bolder, fontSize: xlarger},
	'secondary_label':		{},
	'tertiary_label':			{fontSize: smaller},
	'error_label':				{strokeStyle: 'red', fillStyle: 'red', fontWeight: bolder, fontSize: larger},
	
	'good_level':				{strokeStyle: 'black', fillStyle: 'green'},
	'ok_level':					{strokeStyle: 'black', fillStyle: 'deepskyblue'},
	'warn_level':				{strokeStyle: 'black', fillStyle: 'lightsalmon'},
	'danger_level':			{strokeStyle: 'black', fillStyle: 'red'},
};

// Applies cascading styles to the current canvas context.
function apply_styles(context, names)
{
	var style = compose_styles(names);
	
	var font = '';
	font += style.fontStyle + " ";
	font += style.fontWeight + " ";
	font += style.fontSize + "pt ";
	font += style.fontFamily + " ";
	context.font = font;
	
	context.lineWidth = style.lineWidth;
	context.strokeStyle = style.strokeStyle;
	context.fillStyle = style.fillStyle;
	
	return style;
}

function compose_styles(names)
{
	var style = clone(GNOS.styles['default']);
	
	for (var i=0; i < names.length; ++i)
	{
		var name = names[i];
		if (name in GNOS.styles)
		{
			var rhs = GNOS.styles[name];
			for (var key in rhs)
			{
				if (rhs[key] instanceof Function)
					style = rhs[key](style);
				else
					style[key] = rhs[key];
			}
		}
		else
		{
			console.log("'{0}' is not a known style".format(name));
		}
	}
	
	return style;
}

function bolder(style)
{
	var result = clone(style);
	result.fontWeight += 300;
	if (result.fontWeight > 900)
		result.fontWeight = 900;
	return result;
}

function xlarger(style)
{
	var result = clone(style);
	result.fontSize = Math.round(1.4*result.fontSize);
	return result;
}

function larger(style)
{
	var result = clone(style);
	result.fontSize = Math.round(1.2*result.fontSize);
	return result;
}

function smaller(style)
{
	var result = clone(style);
	result.fontSize = Math.max(Math.round(0.8*result.fontSize), 8);
	return result;
}

function xsmaller(style)
{
	var result = clone(style);
	result.fontSize = Math.max(Math.round(0.6*result.fontSize), 8);
	return result;
}


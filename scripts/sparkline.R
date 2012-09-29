# To avoid starting up Rscript multiple times copies of this file are
# concatenated together. The concatenator will add the library import.
#library(YaleToolkit)

samples = c({{samples}});

png("{{file}}", {{width}}, {{height}})

grid.newpage()
pushViewport(viewport(w = 0.85, h = 0.85))	# http://rgm2.lab.nig.ac.jp/RGM2/func.php?rd_id=grid:viewport

# http://rgm2.lab.nig.ac.jp/RGM2/func.php?rd_id=YaleToolkit:sparkline
# Note that labels can be added via the grid.text function (or main and sub arguments here).
sparkline(
	samples,
	ptopts = list(labels = 'min.max'),			# add labels for the min and max samples
	IQR = gpar(fill = 'cornsilk', col = 'cornsilk'),	# show inter-quartile range
	buffer = unit(4, 'points'),
	new = FALSE
)

popViewport()
dev.off()
########################################################


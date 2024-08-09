.PHONY : all
all: triangle

triangle: main.m
	$(CC) main.m -o triangle -I. -framework Cocoa -framework IOKit -framework CoreFoundation -framework QuartzCore

clean:
	rm triangle
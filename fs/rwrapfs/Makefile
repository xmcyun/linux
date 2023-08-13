obj-m := rust_wrapfs.o

KDIR ?= ../../

EXTRA_CFLAGS = -g -DDEBUG

default:
	$(MAKE) -C $(KDIR) LLVM=1 M=$$PWD

clean:
	rm -rf *.o *.mod *.mod.c *.cmd .*.cmd *.ko modules.order Module.symvers


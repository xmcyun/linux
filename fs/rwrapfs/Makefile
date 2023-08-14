obj-m := rust_wrapfs.o

KDIR ?= ../../

EXTRA_CFLAGS = -g -DDEBUG

default:
	$(MAKE) -C $(KDIR) LLVM=1 M=$(PWD) modules

clean:
	$(MAKE) -C $(KDIR) LLVM=1 M=$(PWD) clean


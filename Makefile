arch := x86_64
asm_src_dir := src/arch/${arch}/asm
asm_src_files := $(wildcard ${asm_src_dir}/*.asm)
bin_dir := target/bin
asm_obj_files := $(patsubst ${asm_src_dir}/%.asm, ${bin_dir}/%.o, ${asm_src_files})
rust_bin := target/x86_64-unknown-none/debug/libos.a
conf_dir := config
iso_dir := target/isofiles
kernel := ${bin_dir}/kernel

.PHONY: all
all: ${bin_dir}/os.iso
	qemu-system-x86_64 -boot d -cdrom target/bin/os.iso \
	-m 4G \
	-smp 4 \
	-netdev user,id=n1,hostfwd=tcp::5555-:22 -device rtl8139,netdev=n1 \
	-object filter-dump,id=f1,netdev=n1,file=/tmp/dump.pcap \
	-monitor stdio \
	-d int -M smm=off \
	-no-reboot -no-shutdown \
	-serial file:/tmp/serial \
	-s -S

${bin_dir}/os.iso: ${kernel}
	mkdir -p ${iso_dir}/boot/grub
	cp ${conf_dir}/grub.cfg ${iso_dir}/boot/grub
	cp ${kernel} ${iso_dir}/boot/
	grub-mkrescue -o ${bin_dir}/os.iso ${iso_dir}
	rm -rf ${iso_dir}

${kernel}: ${asm_obj_files} ${rust_bin}
	ld --eh-frame-hdr -verbose -n -o ${kernel} -T ${conf_dir}/linker.ld ${asm_obj_files} ${rust_bin}

.PHONY: ${rust_bin}
${rust_bin}: 
	cargo b

${bin_dir}/%.o: ${asm_src_dir}/%.asm
	@mkdir -p ${bin_dir}
	nasm -f elf64 $< -o $@

.PHONY: clean
clean:
	rm -r ${bin_dir}/*
	
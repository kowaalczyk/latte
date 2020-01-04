define i32 @__func__main(  ){
	%1 = icmp eq i1 true, true
	br i1 %1, label %__branch_true__1, label %__branch_false__1
__branch_true__1:
	call void @printInt (i32 42)
	br label %__branch_end__1
__branch_false__1:
	br label %__branch_end__1
__branch_end__1:
	ret i32 0
}
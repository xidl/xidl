pub(crate) const ANNOTATION_CASES: &[(&str, &str)] = &[(
    "annotation_basic",
    r#"
        @id(1)
        struct S {
            @id(10) long a; //@id(11)
            @optional short b;
        };

        @my::anno(abc=1)
        enum E { @id(0) A, @id(1) B };

        @id(2)
        bitmask BM { @id(0) A, @id(1) B };

        @id(3)
        union U switch (long) {
            case 0: @id(100) long a;
            default: long b;
        };

        @id(4)
        interface I {
            @oneway void ping();
            @key attribute long value;
        };
    "#,
)];

pub(crate) const BITMASK_CASES: &[(&str, &str)] = &[(
    "bitmask_def",
    r#"
        bitmask A {};
        bitmask A { A, B, C};
        bitmask A { A, B, C,};
    "#,
)];

pub(crate) const BITSET_CASES: &[(&str, &str)] = &[(
    "bitset_def",
    r#"
        bitset A {};
        bitset A: B {};
        bitset A: B {
            bitfield<1> a;
            bitfield<1> a b c;
        };
    "#,
)];

pub(crate) const CONST_CASES: &[(&str, &str)] = &[
    ("const_dec", "const int32 a = 10;"),
    ("const_binary", "const int32 a = 0b10;"),
    ("const_oct", "const int32 a = 0o10;"),
    ("const_hex", "const int32 a = 0xff;"),
    (
        "const_scoped_name",
        r#"
            const int a = 0;
            const uint8 a = 0;
            const uint16 a = 0;
            const uint32 a = 0;
            const uint64 a = 0;

            const int8 a = 0;
            const int16 a = 0;
            const int32 a = 0;
            const int64 a = 0;

            const char8 a = 0;
            const char16 a = 0;

            const char C1 = 'X';
            const wchar C2 = L'X';
            const string C3 = "aaa";
            const wstring C3 = L"aaa";
            const bool C3 = false;

            const ::A a = 0;
            const A::B a = 0;
            const A::B::C a = 0;
            const A::B::C::D::E::F a = 0;

            const M::Size MYSIZE = M::medium;

            const float const_float = 13.1;
            const double const_double = 84.1e;
            const long double const_longdouble = 46.1;
        "#,
    ),
];

pub(crate) const ENUM_CASES: &[(&str, &str)] = &[
    ("enum_empty", "enum A { };"),
    ("enum_simple", "enum A { B, C };"),
    ("enum_simple_comma", "enum A { B, C, };"),
];

pub(crate) const EXCEPT_CASES: &[(&str, &str)] = &[(
    "except_dcl",
    r#"
        exception HelloWorld {
            u8 a;
            u16 b[10];
            string c[10][20];
            sequence<u8> c;
            string<20> d;
            wstring<20> d;
            fixed<1,2> d;
        };
    "#,
)];

pub(crate) const INTERFACE_CASES: &[(&str, &str)] = &[(
    "interface_dcl",
    r#"
        interface HelloWorld;
        interface HelloWorld {};

        interface HelloWorld: Parent {};
        interface HelloWorld: Parent1, Parent2, Parent3 {};

        interface A: B, C, D {
            void func1();
            void func1() raises(A);
            void func1() raises(A,B,C);
            void func1(in u8 attr, out u16 attr);
            void func1(in u8 attr, out u16 attr) raises(A);
            void func1(in u8 attr, out u16 attr) raises(A,B,C);
            readonly attribute u8 attr1, attr2, attr3;
            readonly attribute u8 attr1 raises(A);
            readonly attribute u8 attr1 raises(A, B, C);
            attribute u8 attr1, attr2, attr3;
            attribute u8 attr1 getraises(A);
            attribute u8 attr1 getraises(A, B, C) setraises(A);
            attribute u8 attr1 setraises(A);
        };

        interface A {
            typedef long L1; // idl 7.4.4
            const int a = 10; // idl 7.4.4
            exception A { // idl 7.4.4
                int a;
            };
            short opA(in L1 l_1);
        };
    "#,
)];

pub(crate) const MISC_CASES: &[(&str, &str)] = &[(
    "misc",
    r#"
        enum Color { red, green, blue };
        const Color FAVORITE_COLOR = red;

        module M {
            enum Size { small, medium, large };
        };

        const M::Size MYSIZE = M::medium;
        const Color col = red;
        const Color another = M::medium;

        // 7.4.1.4.4.4.4
        struct Foo; // Forward declaration
        typedef sequence<Foo> FooSeq;
        typedef sequence<Foo, 12> FooSeq;
        typedef sequence<Foo, red> FooSeq;

        struct Foo {
            long value;
            FooSeq chain; // Recursive
        };

        union Bar; // Forward declaration
        typedef sequence<Bar> BarSeq;

        union Bar switch (long) { // Define incomplete union
            case 0:
                long l_mem;
            case 1:
                struct Foo {
                    double d_mem;
                    BarSeq nested; // OK, recurse on enclosing incomplete type
                } s_mem;
        };

        // 7.4.3.4.3.2.1
        interface A { };
        interface B: A { };
        interface C: A { };
        interface D: B, C { }; // OK
        interface E: A, B { }; // OK

        interface A {
            void make_it_so();
        };
        interface B: A {
            short make_it_so(in long times); // Error: redefinition of make_it_so
        };

        module Example {
            interface base; // Forward declaration
            // ...
            interface derived : base {}; // Error
            interface base {}; // Define base
            interface derived : base {}; // OK
        };

        // 7.4.4.4

        interface A {
            typedef long L1;
            short opA (in L1 l_1);
        };
        interface B {
            typedef short L1;
            L1 opB (in long l);
        };
        interface C: B, A {
            typedef L1 L2; // Error: L1 ambiguous
            typedef A::L1 L3; // A::L1 is OK
            B::L1 opC (in L3 l_3); // All OK no ambiguities
        };

        const long L = 3;
        interface A {
            typedef float coord[1];
            void f (in coord s); // s has three floats
        };
        interface B {
            const long L = 4;
        };
        interface C: B, A { }; // What is C::f()'s signature?

        interface A {
            typedef string<128> string_t;
        };
        interface B {
            typedef string<256> string_t;
        };
        interface C: A, B {
            attribute string_t Title; // Error: string_t ambiguous
            attribute A::string_t Name; // OK
            attribute B::string_t City; // OK
        };
    "#,
)];

pub(crate) const MODULE_CASES: &[(&str, &str)] = &[(
    "module_dcl",
    r#"
        module A {};

        module B {
        const u8 a = 10;
        struct B;
        };
    "#,
)];

pub(crate) const PREPROC_CASES: &[(&str, &str)] = &[(
    "preproc",
    r#"
        #include "aaaa"

        #ifdef BASIC

        module A {};

        #program once

        module A {}; #endif
    "#,
)];

pub(crate) const STRUCT_CASES: &[(&str, &str)] = &[
    ("struct_empty", ""),
    ("struct_simple", "struct A;"),
    (
        "struct_derive",
        r#"
        @derive(Debug)
        struct A {};

        @derive(Debug, Serialize)
        struct A {};
    "#,
    ),
    (
        "struct_def",
        r#"
            struct A {};
            struct A {
                int32 a;
            };
            struct A {
                ::A::b a;
            };
            struct A: B {};

            struct _A {};

            struct _Custom {
                Inner var_inner;
            };

            struct HelloWorld {
                u8 a;
                u16 b[10];
                string c[10][20];
                sequence<u8> c;
                string<20> d;
                wstring<20> d;
                // fixed<1,2> d;
                any d;
            };
        "#,
    ),
];

pub(crate) const TEMPLATE_MODULE_CASES: &[(&str, &str)] = &[(
    "template_module_dcl",
    r#"
        module MyTemplModule <typename T, struct S> {
        };

        module MyTemplModule <typename T, struct S, ::A a, A::B::C::D a> {
        };

        module MyTemplModule <typename T, struct S, long m> {
            alias MyTemplModule<T2, S2, m> MyTemplModule;
            interface Bar : A::Foo {};
        };
    "#,
)];

pub(crate) const TYPEDEF_CASES: &[(&str, &str)] = &[(
    "typedef_dcl",
    r#"
        typedef sequence<Foo> FooSeq;
        typedef u8 uint8_t;
        typedef string u8string;
        typedef wstring u16string;
    "#,
)];

pub(crate) const UNION_CASES: &[(&str, &str)] = &[(
    "union_dcl",
    r#"
        union A;
        union B switch (int32) {};
        union C switch (int32) {
            case 0:
                int32 a;
            case 1:
                string b;
        };
    "#,
)];

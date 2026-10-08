use vb6_compiler::ast::expr::BinaryOperator;
use vb6_compiler::ir::compile_to_ir;
use vb6_compiler::parse_source;
use vb6_compiler::semantic::{SemanticAnalyzer, SemanticError};
use vb6_compiler::types::*;

#[test]
fn test_type_sizes_and_alignment_32_and_64_bit() {
    // 32-bit sizes
    assert_eq!(VbType::Byte.size_in_bytes(false), 1);
    assert_eq!(VbType::Integer.size_in_bytes(false), 2);
    assert_eq!(VbType::Long.size_in_bytes(false), 4);
    assert_eq!(VbType::LongPtr.size_in_bytes(false), 4);
    assert_eq!(VbType::LongLong.size_in_bytes(false), 8);
    assert_eq!(VbType::Single.size_in_bytes(false), 4);
    assert_eq!(VbType::Double.size_in_bytes(false), 8);
    assert_eq!(VbType::Boolean.size_in_bytes(false), 2);
    assert_eq!(VbType::Currency.size_in_bytes(false), 8);
    assert_eq!(VbType::Date.size_in_bytes(false), 8);
    assert_eq!(VbType::String.size_in_bytes(false), 4); // 32-bit BSTR pointer
    assert_eq!(VbType::FixedString(30).size_in_bytes(false), 30);
    assert_eq!(VbType::Variant.size_in_bytes(false), 16); // 16 bytes on x86
    assert_eq!(VbType::Object.size_in_bytes(false), 4);
    assert_eq!(VbType::Array(Box::new(VbType::Long)).size_in_bytes(false), 4);
    assert_eq!(VbType::Enum("Colors".to_string()).size_in_bytes(false), 4);

    // 64-bit sizes (x64 Windows)
    assert_eq!(VbType::Byte.size_in_bytes(true), 1);
    assert_eq!(VbType::Integer.size_in_bytes(true), 2);
    assert_eq!(VbType::Long.size_in_bytes(true), 4);
    assert_eq!(VbType::LongPtr.size_in_bytes(true), 8); // 64-bit pointer
    assert_eq!(VbType::LongLong.size_in_bytes(true), 8);
    assert_eq!(VbType::Single.size_in_bytes(true), 4);
    assert_eq!(VbType::Double.size_in_bytes(true), 8);
    assert_eq!(VbType::Boolean.size_in_bytes(true), 2);
    assert_eq!(VbType::Currency.size_in_bytes(true), 8);
    assert_eq!(VbType::Date.size_in_bytes(true), 8);
    assert_eq!(VbType::String.size_in_bytes(true), 8); // 64-bit pointer
    assert_eq!(VbType::Variant.size_in_bytes(true), 24); // 24 bytes on x64
    assert_eq!(VbType::Object.size_in_bytes(true), 8);
    assert_eq!(VbType::Array(Box::new(VbType::Long)).size_in_bytes(true), 8);

    // Natural Alignments
    assert_eq!(VbType::Byte.alignment(true), 1);
    assert_eq!(VbType::Integer.alignment(true), 2);
    assert_eq!(VbType::Long.alignment(true), 4);
    assert_eq!(VbType::LongPtr.alignment(true), 8);
    assert_eq!(VbType::Double.alignment(true), 8);
    assert_eq!(VbType::Currency.alignment(true), 8);
}

#[test]
fn test_numeric_ranges_and_overflow_validation() {
    assert!(VbType::Byte.can_hold_integer(0));
    assert!(VbType::Byte.can_hold_integer(255));
    assert!(!VbType::Byte.can_hold_integer(256));
    assert!(!VbType::Byte.can_hold_integer(-1));

    assert!(VbType::Integer.can_hold_integer(-32768));
    assert!(VbType::Integer.can_hold_integer(32767));
    assert!(!VbType::Integer.can_hold_integer(32768));

    // Semantic analyzer detection of literal overflow
    let source = r#"
Sub TestOverflow()
    Dim B As Byte
    B = 300
End Sub
"#;
    let ast = parse_source(source).expect("Failed to parse");
    let mut analyzer = SemanticAnalyzer::new();
    let res = analyzer.analyze_file(&ast);
    assert!(res.is_err(), "Expected overflow error for Byte = 300");
    let errors = res.unwrap_err();
    assert!(
        errors.iter().any(|e| matches!(e, SemanticError::Overflow { target_type: VbType::Byte, .. })),
        "Expected SemanticError::Overflow on Byte"
    );
}

#[test]
fn test_binary_operator_promotion_rules() {
    // Byte + Byte -> Integer (VB6 behavior)
    let add_res = promote_binary_operator(&VbType::Byte, &BinaryOperator::Add, &VbType::Byte).unwrap();
    assert_eq!(add_res, VbType::Integer);

    // Integer + Long -> Long
    let add_long = promote_binary_operator(&VbType::Integer, &BinaryOperator::Add, &VbType::Long).unwrap();
    assert_eq!(add_long, VbType::Long);

    // Long + Single -> Single
    let add_single = promote_binary_operator(&VbType::Long, &BinaryOperator::Add, &VbType::Single).unwrap();
    assert_eq!(add_single, VbType::Single);

    // Single + Double -> Double
    let add_double = promote_binary_operator(&VbType::Single, &BinaryOperator::Add, &VbType::Double).unwrap();
    assert_eq!(add_double, VbType::Double);

    // Floating division '/' always returns Double
    let div_res = promote_binary_operator(&VbType::Long, &BinaryOperator::Divide, &VbType::Long).unwrap();
    assert_eq!(div_res, VbType::Double);

    // Single / Single -> Single
    let div_single = promote_binary_operator(&VbType::Single, &BinaryOperator::Divide, &VbType::Single).unwrap();
    assert_eq!(div_single, VbType::Single);

    // Integer division '\' always returns Long
    let int_div = promote_binary_operator(&VbType::Integer, &BinaryOperator::IntegerDivide, &VbType::Integer).unwrap();
    assert_eq!(int_div, VbType::Long);

    // String concat '&' always returns String
    let concat = promote_binary_operator(&VbType::Long, &BinaryOperator::Concat, &VbType::Double).unwrap();
    assert_eq!(concat, VbType::String);

    // Comparison '<>' always returns Boolean
    let cmp = promote_binary_operator(&VbType::Long, &BinaryOperator::NotEqual, &VbType::Long).unwrap();
    assert_eq!(cmp, VbType::Boolean);

    // Logical And on Boolean -> Boolean
    let log_and = promote_binary_operator(&VbType::Boolean, &BinaryOperator::And, &VbType::Boolean).unwrap();
    assert_eq!(log_and, VbType::Boolean);

    // Bitwise And on Integers -> Long
    let bit_and = promote_binary_operator(&VbType::Integer, &BinaryOperator::And, &VbType::Long).unwrap();
    assert_eq!(bit_and, VbType::Long);
}

#[test]
fn test_udt_layout_computation() {
    let members = vec![
        ("Flag".to_string(), VbType::Byte),
        ("Id".to_string(), VbType::Long),
        ("Count".to_string(), VbType::Integer),
    ];

    // Compute layout on 32-bit
    let layout32 = UdtLayout::compute("SampleUdt", &members, false);
    assert_eq!(layout32.name, "SampleUdt");
    assert_eq!(layout32.members[0].name, "Flag");
    assert_eq!(layout32.members[0].offset, 0);
    assert_eq!(layout32.members[0].size, 1);

    // "Id" has alignment 4, so offset should be aligned from 1 up to 4
    assert_eq!(layout32.members[1].name, "Id");
    assert_eq!(layout32.members[1].offset, 4);
    assert_eq!(layout32.members[1].size, 4);

    // "Count" has alignment 2, offset 8 is already aligned
    assert_eq!(layout32.members[2].name, "Count");
    assert_eq!(layout32.members[2].offset, 8);
    assert_eq!(layout32.members[2].size, 2);

    // Total size padded to max alignment (4): 10 padded to 12
    assert_eq!(layout32.total_size, 12);
    assert_eq!(layout32.alignment, 4);
}

#[test]
fn test_enum_auto_increment_values() {
    let raw = vec![
        ("Red".to_string(), Some(1)),
        ("Green".to_string(), None),
        ("Blue".to_string(), Some(10)),
        ("Yellow".to_string(), None),
    ];

    let enum_def = EnumDefinition::new("Colors", &raw);
    assert_eq!(enum_def.get_value("Red"), Some(1));
    assert_eq!(enum_def.get_value("Green"), Some(2));
    assert_eq!(enum_def.get_value("Blue"), Some(10));
    assert_eq!(enum_def.get_value("Yellow"), Some(11));
    assert_eq!(enum_def.get_value("NonExistent"), None);
}

#[test]
fn test_parameter_semantics_byval_and_byref() {
    // Valid ByVal call with coercible integer to long
    let valid_byval = r#"
Sub Process(ByVal X As Long)
End Sub

Sub Main()
    Dim I As Integer
    I = 5
    Process I
End Sub
"#;
    let ast = parse_source(valid_byval).expect("Parse error");
    let mut analyzer = SemanticAnalyzer::new();
    assert!(analyzer.analyze_file(&ast).is_ok());

    // ByRef call passing incompatible variable type directly
    let invalid_byref = r#"
Sub ProcessByRef(ByRef X As Long)
End Sub

Sub Main()
    Dim I As Integer
    I = 5
    ProcessByRef I
End Sub
"#;
    let ast_err = parse_source(invalid_byref).expect("Parse error");
    let mut analyzer_err = SemanticAnalyzer::new();
    let res = analyzer_err.analyze_file(&ast_err);
    assert!(res.is_err(), "Expected ByRefTypeMismatch");
    let errors = res.unwrap_err();
    assert!(
        errors.iter().any(|e| matches!(e, SemanticError::ByRefTypeMismatch { .. })),
        "Expected SemanticError::ByRefTypeMismatch"
    );
}

#[test]
fn test_parameter_optional_rules() {
    // Calling procedure with omitted optional argument
    let valid_optional = r#"
Sub Config(ByVal Name As String, Optional ByVal Timeout As Long = 30)
End Sub

Sub Main()
    Config "Database"
    Config "Database", 60
End Sub
"#;
    let ast = parse_source(valid_optional).expect("Parse error");
    let mut analyzer = SemanticAnalyzer::new();
    assert!(analyzer.analyze_file(&ast).is_ok(), "Optional argument should be omissible");

    // Invalid declaration: Optional parameter preceding a required parameter
    let invalid_order = r#"
Sub BadProc(Optional ByVal X As Long = 10, ByVal Y As Long)
End Sub
"#;
    let ast_bad = parse_source(invalid_order).expect("Parse error");
    let mut analyzer_bad = SemanticAnalyzer::new();
    let res = analyzer_bad.analyze_file(&ast_bad);
    assert!(res.is_err(), "Expected OptionalParamMustFollowRequired");
    let errors = res.unwrap_err();
    assert!(
        errors.iter().any(|e| matches!(e, SemanticError::OptionalParamMustFollowRequired { .. })),
        "Expected SemanticError::OptionalParamMustFollowRequired"
    );
}

#[test]
fn test_parameter_paramarray_rules() {
    // Valid ParamArray definition and variadic call
    let valid_paramarray = r#"
Sub PrintAll(ByVal Prefix As String, ParamArray Items())
End Sub

Sub Main()
    PrintAll "Zero"
    PrintAll "One", 100
    PrintAll "Three", 10, "Hello", True
End Sub
"#;
    let ast = parse_source(valid_paramarray).expect("Parse error");
    let mut analyzer = SemanticAnalyzer::new();
    assert!(analyzer.analyze_file(&ast).is_ok(), "ParamArray should accept variadic arguments");

    // Invalid: ParamArray not at the end
    let invalid_paramarray = r#"
Sub BadParamArray(ParamArray Items(), ByVal Extra As Long)
End Sub
"#;
    let ast_err = parse_source(invalid_paramarray).expect("Parse error");
    let mut analyzer_err = SemanticAnalyzer::new();
    let res = analyzer_err.analyze_file(&ast_err);
    assert!(res.is_err(), "Expected ParamArrayMustBeLast");
    let errors = res.unwrap_err();
    assert!(
        errors.iter().any(|e| matches!(e, SemanticError::ParamArrayMustBeLast { .. })),
        "Expected SemanticError::ParamArrayMustBeLast"
    );
}

#[test]
fn test_ir_lowering_preserves_parameters() {
    let source = r#"
Sub Compute(ByVal A As Long, ByRef B As Double, Optional ByVal C As Integer = 10, ParamArray Rest())
    Dim Total As Double
    Total = A + B
End Sub
"#;
    let ast = parse_source(source).expect("Parse error");
    let ir = compile_to_ir(&ast, "ParamModule");
    assert_eq!(ir.functions.len(), 1);

    let func = &ir.functions[0];
    assert_eq!(func.name, "Compute");
    assert_eq!(func.params.len(), 4);

    assert_eq!(func.params[0].name, "A");
    assert!(func.params[0].is_by_val);
    assert_eq!(func.params[0].vb_type, VbType::Long);

    assert_eq!(func.params[1].name, "B");
    assert!(!func.params[1].is_by_val); // ByRef
    assert_eq!(func.params[1].vb_type, VbType::Double);

    assert_eq!(func.params[2].name, "C");
    assert!(func.params[2].is_optional);
    assert!(func.params[2].is_by_val);
    assert_eq!(func.params[2].vb_type, VbType::Integer);

    assert_eq!(func.params[3].name, "Rest");
    assert!(func.params[3].is_param_array);
}

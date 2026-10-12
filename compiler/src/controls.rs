//! Built-in intrinsic VB6 controls and Form class definitions.
//! Clean-room implementation matching Microsoft VB6 TypeLib IDL specifications.

use crate::ast::decl::Declaration;
use std::sync::OnceLock;

pub const BUILTIN_CONTROLS_SOURCE: &str = r#"
Class CommandButton
    Public Caption As String
    Public Value As Boolean
    Public Enabled As Boolean
    Public Visible As Boolean
    Public Left As Long
    Public Top As Long
    Public Width As Long
    Public Height As Long
    Public hWnd As LongLong
    Public Tag As String
    Public Name As String
    Public Index As Long
    Public Sub SetFocus()
    End Sub
    Public Sub Refresh()
    End Sub
End Class

Class TextBox
    Public Text As String
    Public Locked As Boolean
    Public MultiLine As Boolean
    Public Enabled As Boolean
    Public Visible As Boolean
    Public Left As Long
    Public Top As Long
    Public Width As Long
    Public Height As Long
    Public hWnd As LongLong
    Public Tag As String
    Public Name As String
    Public Index As Long
    Public Sub SetFocus()
    End Sub
    Public Sub Refresh()
    End Sub
End Class

Class Label
    Public Caption As String
    Public Enabled As Boolean
    Public Visible As Boolean
    Public Left As Long
    Public Top As Long
    Public Width As Long
    Public Height As Long
    Public hWnd As LongLong
    Public Tag As String
    Public Name As String
    Public Index As Long
    Public Sub Refresh()
    End Sub
End Class

Class CheckBox
    Public Caption As String
    Public Value As Long
    Public Enabled As Boolean
    Public Visible As Boolean
    Public Left As Long
    Public Top As Long
    Public Width As Long
    Public Height As Long
    Public hWnd As LongLong
    Public Tag As String
    Public Name As String
    Public Index As Long
    Public Sub Refresh()
    End Sub
End Class

Class OptionButton
    Public Caption As String
    Public Value As Boolean
    Public Enabled As Boolean
    Public Visible As Boolean
    Public Left As Long
    Public Top As Long
    Public Width As Long
    Public Height As Long
    Public hWnd As LongLong
    Public Tag As String
    Public Name As String
    Public Index As Long
    Public Sub Refresh()
    End Sub
End Class

Class Frame
    Public Caption As String
    Public Enabled As Boolean
    Public Visible As Boolean
    Public Left As Long
    Public Top As Long
    Public Width As Long
    Public Height As Long
    Public hWnd As LongLong
    Public Tag As String
    Public Name As String
    Public Index As Long
    Public Sub Refresh()
    End Sub
End Class

Class Timer
    Public Interval As Long
    Public Enabled As Boolean
    Public Tag As String
    Public Name As String
    Public Index As Long
    Public hWnd As LongLong
End Class

Class Form
    Public Caption As String
    Public Enabled As Boolean
    Public Visible As Boolean
    Public Left As Long
    Public Top As Long
    Public Width As Long
    Public Height As Long
    Public hWnd As LongLong
    Public Tag As String
    Public Name As String
    Public Sub Show(Optional ByVal Modal As Variant, Optional ByVal OwnerForm As Variant)
    End Sub
    Public Sub Hide()
    End Sub
    Public Sub Refresh()
    End Sub
    Public Sub Cls()
    End Sub
End Class
"#;

static BUILTIN_DECLARATIONS: OnceLock<Vec<Declaration>> = OnceLock::new();

/// Returns pre-parsed AST declarations for all standard intrinsic controls.
pub fn get_builtin_control_declarations() -> Vec<Declaration> {
    BUILTIN_DECLARATIONS
        .get_or_init(|| {
            let ast = crate::parse_source(BUILTIN_CONTROLS_SOURCE)
                .expect("Failed to parse builtin control classes");
            ast.declarations
        })
        .clone()
}

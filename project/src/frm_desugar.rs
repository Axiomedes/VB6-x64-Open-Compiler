//! Desugarer for VB6 Form files (.frm).
//! Transforms visual Form hierarchy and properties into a strongly-typed VB6 Class definition.

use crate::models::{ControlNode, ControlPropertyValue, FormModule};

pub struct FRMDesugarer;

impl FRMDesugarer {
    /// Desugar a FormModule into VB6 source code defining the Form class and its global instance.
    pub fn desugar(form: &FormModule) -> String {
        let form_name = &form.name;

        // Collect all controls recursively
        let mut controls = Vec::new();
        if let Some(ref root) = form.root_control {
            Self::collect_controls(root, &mut controls);
        }

        // Extract form properties
        let form_caption = Self::get_form_prop(form, "Caption").unwrap_or_else(|| form_name.clone());
        let form_left = Self::get_form_prop(form, "ClientLeft")
            .or_else(|| Self::get_form_prop(form, "Left"))
            .unwrap_or_else(|| "1200".to_string());
        let form_top = Self::get_form_prop(form, "ClientTop")
            .or_else(|| Self::get_form_prop(form, "Top"))
            .unwrap_or_else(|| "800".to_string());
        let form_width = Self::get_form_prop(form, "ClientWidth")
            .or_else(|| Self::get_form_prop(form, "Width"))
            .unwrap_or_else(|| "4800".to_string());
        let form_height = Self::get_form_prop(form, "ClientHeight")
            .or_else(|| Self::get_form_prop(form, "Height"))
            .unwrap_or_else(|| "3600".to_string());

        let mut out = String::new();

        out.push_str(&format!("Class {}\n", form_name));
        out.push_str("    ' Standard Form visual properties\n");
        out.push_str("    Public Caption As String\n");
        out.push_str("    Public Left As Long\n");
        out.push_str("    Public Top As Long\n");
        out.push_str("    Public Width As Long\n");
        out.push_str("    Public Height As Long\n");
        out.push_str("    Public Visible As Boolean\n");
        out.push_str("    Public Enabled As Boolean\n");
        out.push_str("    Public hWnd As LongLong\n");
        out.push_str("    Public Tag As String\n");
        out.push_str("    Public Name As String\n\n");

        out.push_str("    ' Control member declarations\n");
        for c in &controls {
            let ctrl_type = Self::normalize_control_type(&c.control_type);
            out.push_str(&format!("    Public {} As {}\n", c.name, ctrl_type));
        }
        out.push('\n');

        out.push_str("    ' Standard Form methods\n");
        out.push_str("    Public Sub Show(Optional ByVal Modal As Variant, Optional ByVal OwnerForm As Variant)\n");
        out.push_str("        If hWnd = 0 Then\n");
        out.push_str("            hWnd = vb6_form_create(Caption, Left, Top, Width, Height)\n");
        out.push_str("            BindControls\n");
        out.push_str("        End If\n");
        out.push_str("        Visible = True\n");
        out.push_str("        vb6_form_show hWnd, 0\n");
        out.push_str("        Form_Load\n");
        out.push_str("    End Sub\n\n");

        out.push_str("    Public Sub Hide()\n");
        out.push_str("        Dim Cancel As Integer\n");
        out.push_str("        Cancel = 0\n");
        out.push_str("        Form_Unload Cancel\n");
        out.push_str("        Visible = False\n");
        out.push_str("        If hWnd <> 0 Then\n");
        out.push_str("            vb6_form_hide hWnd\n");
        out.push_str("        End If\n");
        out.push_str("    End Sub\n\n");

        out.push_str("    Public Sub Refresh()\n");
        out.push_str("        vb6_do_events\n");
        out.push_str("    End Sub\n\n");

        out.push_str("    Public Sub Move(ByVal nLeft As Long, Optional ByVal nTop As Variant, Optional ByVal nWidth As Variant, Optional ByVal nHeight As Variant)\n");
        out.push_str("        Left = nLeft\n");
        out.push_str("        If hWnd <> 0 Then\n");
        out.push_str("            vb6_control_set_bounds hWnd, Left, Top, Width, Height\n");
        out.push_str("        End If\n");
        out.push_str("    End Sub\n\n");

        out.push_str("    Public Sub Cls()\n");
        out.push_str("    End Sub\n\n");

        // BindControls: Win32 native control handles
        out.push_str("    Private Sub BindControls()\n");
        out.push_str("        If hWnd <> 0 Then\n");
        for c in &controls {
            let ctrl_type = Self::normalize_control_type(&c.control_type);
            match ctrl_type.as_str() {
                "CommandButton" => {
                    out.push_str(&format!(
                        "            {}.hWnd = vb6_control_create_button(hWnd, {}.Caption, {}.Left, {}.Top, {}.Width, {}.Height)\n",
                        c.name, c.name, c.name, c.name, c.name, c.name
                    ));
                }
                "TextBox" => {
                    out.push_str(&format!(
                        "            {}.hWnd = vb6_control_create_textbox(hWnd, {}.Text, {}.Left, {}.Top, {}.Width, {}.Height, 0)\n",
                        c.name, c.name, c.name, c.name, c.name, c.name
                    ));
                }
                "Label" => {
                    out.push_str(&format!(
                        "            {}.hWnd = vb6_control_create_label(hWnd, {}.Caption, {}.Left, {}.Top, {}.Width, {}.Height)\n",
                        c.name, c.name, c.name, c.name, c.name, c.name
                    ));
                }
                "CheckBox" => {
                    out.push_str(&format!(
                        "            {}.hWnd = vb6_control_create_checkbox(hWnd, {}.Caption, {}.Left, {}.Top, {}.Width, {}.Height)\n",
                        c.name, c.name, c.name, c.name, c.name, c.name
                    ));
                }
                "OptionButton" => {
                    out.push_str(&format!(
                        "            {}.hWnd = vb6_control_create_option(hWnd, {}.Caption, {}.Left, {}.Top, {}.Width, {}.Height)\n",
                        c.name, c.name, c.name, c.name, c.name, c.name
                    ));
                }
                "Frame" => {
                    out.push_str(&format!(
                        "            {}.hWnd = vb6_control_create_frame(hWnd, {}.Caption, {}.Left, {}.Top, {}.Width, {}.Height)\n",
                        c.name, c.name, c.name, c.name, c.name, c.name
                    ));
                }
                "Timer" => {
                    out.push_str(&format!(
                        "            {}.hWnd = vb6_control_create_timer(hWnd, {}.Interval, 1)\n",
                        c.name, c.name
                    ));
                }
                _ => {}
            }
        }
        out.push_str("        End If\n");
        out.push_str("    End Sub\n\n");

        // Class_Initialize: instantiate controls and assign initial properties from .frm
        out.push_str("    Private Sub Class_Initialize()\n");
        out.push_str(&format!("        Name = \"{}\"\n", form_name));
        out.push_str(&format!("        Caption = \"{}\"\n", Self::escape_str(&form_caption)));
        out.push_str(&format!("        Left = {}\n", form_left));
        out.push_str(&format!("        Top = {}\n", form_top));
        out.push_str(&format!("        Width = {}\n", form_width));
        out.push_str(&format!("        Height = {}\n", form_height));
        out.push_str("        Visible = False\n");
        out.push_str("        Enabled = True\n\n");

        for c in &controls {
            let ctrl_type = Self::normalize_control_type(&c.control_type);
            out.push_str(&format!("        Set {} = New {}\n", c.name, ctrl_type));
            out.push_str(&format!("        {}.Name = \"{}\"\n", c.name, c.name));

            if let Some(val) = Self::get_ctrl_prop(c, "Caption") {
                out.push_str(&format!("        {}.Caption = \"{}\"\n", c.name, Self::escape_str(&val)));
            }
            if let Some(val) = Self::get_ctrl_prop(c, "Text") {
                out.push_str(&format!("        {}.Text = \"{}\"\n", c.name, Self::escape_str(&val)));
            }
            if ctrl_type != "Timer" {
                if let Some(val) = Self::get_ctrl_prop(c, "Left") {
                    out.push_str(&format!("        {}.Left = {}\n", c.name, val));
                }
                if let Some(val) = Self::get_ctrl_prop(c, "Top") {
                    out.push_str(&format!("        {}.Top = {}\n", c.name, val));
                }
                if let Some(val) = Self::get_ctrl_prop(c, "Width") {
                    out.push_str(&format!("        {}.Width = {}\n", c.name, val));
                }
                if let Some(val) = Self::get_ctrl_prop(c, "Height") {
                    out.push_str(&format!("        {}.Height = {}\n", c.name, val));
                }
            }
            if let Some(val) = Self::get_ctrl_prop(c, "Value") {
                if ctrl_type == "OptionButton" {
                    let bool_val = if val == "1" || val.eq_ignore_ascii_case("True") { "True" } else { "False" };
                    out.push_str(&format!("        {}.Value = {}\n", c.name, bool_val));
                } else {
                    out.push_str(&format!("        {}.Value = {}\n", c.name, val));
                }
            }
            if let Some(val) = Self::get_ctrl_prop(c, "Interval") {
                out.push_str(&format!("        {}.Interval = {}\n", c.name, val));
            }
            if let Some(val) = Self::get_ctrl_prop(c, "Enabled") {
                let bool_val = if val == "0" || val.eq_ignore_ascii_case("False") { "False" } else { "True" };
                out.push_str(&format!("        {}.Enabled = {}\n", c.name, bool_val));
            } else {
                out.push_str(&format!("        {}.Enabled = True\n", c.name));
            }
            if ctrl_type != "Timer" {
                if let Some(val) = Self::get_ctrl_prop(c, "Visible") {
                    let bool_val = if val == "0" || val.eq_ignore_ascii_case("False") { "False" } else { "True" };
                    out.push_str(&format!("        {}.Visible = {}\n", c.name, bool_val));
                } else {
                    out.push_str(&format!("        {}.Visible = True\n", c.name));
                }
            }
            out.push('\n');
        }
        out.push_str("    End Sub\n\n");

        // Code behind form: transform Private Sub to Public Sub for event subs so they can be called
        let cleaned_code = Self::process_code_behind(&form.code);
        out.push_str("    ' Code behind form\n");
        out.push_str(&cleaned_code);
        out.push('\n');

        // Provide default empty Form_Load if not defined in code
        if !Self::has_proc(&form.code, "Form_Load") {
            out.push_str("    Public Sub Form_Load()\n    End Sub\n\n");
        }

        // Provide default empty Form_Unload if not defined in code
        if !Self::has_proc(&form.code, "Form_Unload") {
            out.push_str("    Public Sub Form_Unload(Cancel As Integer)\n    End Sub\n\n");
        }

        out.push_str("End Class\n\n");

        // Module-level global instance
        out.push_str(&format!("Public {} As {}\n", form_name, form_name));

        out
    }

    fn collect_controls(node: &ControlNode, list: &mut Vec<ControlNode>) {
        for child in &node.children {
            list.push(child.clone());
            if !child.children.is_empty() {
                Self::collect_controls(child, list);
            }
        }
    }

    fn normalize_control_type(raw: &str) -> String {
        let trimmed = raw.trim();
        if let Some(stripped) = trimmed.strip_prefix("VB.") {
            stripped.to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn get_form_prop(form: &FormModule, key: &str) -> Option<String> {
        let root = form.root_control.as_ref()?;
        for (k, v) in &root.properties {
            if k.eq_ignore_ascii_case(key) {
                if let ControlPropertyValue::Literal(s) = v {
                    return Some(s.clone());
                }
            }
        }
        None
    }

    fn get_ctrl_prop(node: &ControlNode, key: &str) -> Option<String> {
        for (k, v) in &node.properties {
            if k.eq_ignore_ascii_case(key) {
                if let ControlPropertyValue::Literal(s) = v {
                    return Some(s.clone());
                }
            }
        }
        None
    }

    fn escape_str(s: &str) -> String {
        s.replace('"', "\"\"")
    }

    fn has_proc(code: &str, proc_name: &str) -> bool {
        let needle = format!("Sub {}", proc_name);
        for line in code.lines() {
            let trimmed = line.trim();
            if trimmed.to_ascii_lowercase().contains(&needle.to_ascii_lowercase()) {
                return true;
            }
        }
        false
    }

    fn process_code_behind(code: &str) -> String {
        let mut out = Vec::new();
        for line in code.lines() {
            let trimmed = line.trim_start();
            if trimmed.to_ascii_lowercase().starts_with("private sub ") {
                let rest = &trimmed["private sub ".len()..];
                out.push(format!("    Public Sub {}", rest));
            } else if trimmed.to_ascii_lowercase().starts_with("sub ") {
                let rest = &trimmed["sub ".len()..];
                out.push(format!("    Public Sub {}", rest));
            } else {
                out.push(format!("    {}", trimmed));
            }
        }
        out.join("\r\n")
    }
}

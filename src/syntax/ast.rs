//! A typed layer over the red-green tree.
//!
//! The AST is rooted in the [`Markup`] node.

use std::ops::Deref;

use super::{Green, GreenData, NodeKind, RedNode, RedRef, Span};
use crate::geom::{AngleUnit, LengthUnit};
use crate::util::EcoString;

/// A typed AST node.
pub trait TypedNode: Sized {
    /// Convert from a red node to a typed node.
    fn from_red(value: RedRef) -> Option<Self>;

    /// A reference to the underlying red node.
    fn as_red(&self) -> RedRef<'_>;

    /// The source code location.
    fn span(&self) -> Span {
        self.as_red().span()
    }
}

macro_rules! node {
    ($(#[$attr:meta])* $name:ident) => {
        node!{$(#[$attr])* $name: $name}
    };
    ($(#[$attr:meta])* $name:ident: $variant:ident) => {
        node!{$(#[$attr])* $name: NodeKind::$variant}
    };
    ($(#[$attr:meta])* $name:ident: $variants:pat) => {
        #[derive(Debug, Clone, PartialEq, Hash)]
        #[repr(transparent)]
        $(#[$attr])*
        pub struct $name(RedNode);

        impl TypedNode for $name {
            fn from_red(node: RedRef) -> Option<Self> {
                if matches!(node.kind(), $variants) {
                    Some(Self(node.own()))
                } else {
                    None
                }
            }

            fn as_red(&self) -> RedRef<'_> {
                self.0.as_ref()
            }
        }
    };
}

node! {
    /// The syntactical root capable of representing a full parsed document.
    Markup: NodeKind::Markup(_)
}

impl Markup {
    /// The markup nodes.
    pub fn nodes(&self) -> impl Iterator<Item = MarkupNode> + '_ {
        self.0.children().filter_map(|node| match node.kind() {
            NodeKind::Space(2 ..) => Some(MarkupNode::Parbreak),
            NodeKind::Space(_) => Some(MarkupNode::Space),
            NodeKind::Linebreak(j) => Some(MarkupNode::Linebreak(*j)),
            NodeKind::Text(s) => Some(MarkupNode::Text(s.clone())),
            NodeKind::Escape(c) => Some(MarkupNode::Text((*c).into())),
            NodeKind::NonBreakingSpace => Some(MarkupNode::Text('\u{00A0}'.into())),
            NodeKind::Shy => Some(MarkupNode::Text('\u{00AD}'.into())),
            NodeKind::EnDash => Some(MarkupNode::Text('\u{2013}'.into())),
            NodeKind::EmDash => Some(MarkupNode::Text('\u{2014}'.into())),
            NodeKind::Ellipsis => Some(MarkupNode::Text('\u{2026}'.into())),
            NodeKind::Quote(d) => Some(MarkupNode::Quote(*d)),
            NodeKind::Strong => node.cast().map(MarkupNode::Strong),
            NodeKind::Emph => node.cast().map(MarkupNode::Emph),
            NodeKind::Raw(raw) => Some(MarkupNode::Raw(raw.as_ref().clone())),
            NodeKind::Math(math) => Some(MarkupNode::Math(math.as_ref().clone())),
            NodeKind::Heading => node.cast().map(MarkupNode::Heading),
            NodeKind::List => node.cast().map(MarkupNode::List),
            NodeKind::Enum => node.cast().map(MarkupNode::Enum),
            _ => node.cast().map(MarkupNode::Expr),
        })
    }
}

/// A single piece of markup.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkupNode {
    /// Whitespace containing less than two newlines.
    Space,
    /// A forced line break. If `true` (`\`), the preceding line can still be
    /// justified, if `false` (`\+`) not.
    Linebreak(bool),
    /// A paragraph break: Two or more newlines.
    Parbreak,
    /// Plain text.
    Text(EcoString),
    /// A smart quote: `'` (`false`) or `"` (true).
    Quote(bool),
    /// Strong content: `*Strong*`.
    Strong(StrongNode),
    /// Emphasized content: `_Emphasized_`.
    Emph(EmphNode),
    /// A raw block with optional syntax highlighting: `` `...` ``.
    Raw(RawNode),
    /// A math formula: `$a^2 = b^2 + c^2$`.
    Math(MathNode),
    /// A section heading: `= Introduction`.
    Heading(HeadingNode),
    /// An item in an unordered list: `- ...`.
    List(ListNode),
    /// An item in an enumeration (ordered list): `1. ...`.
    Enum(EnumNode),
    /// An expression.
    Expr(Expr),
}

node! {
    /// Strong content: `*Strong*`.
    StrongNode: Strong
}

impl StrongNode {
    /// The contents of the strong node.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("strong node is missing markup body")
    }
}

node! {
    /// Emphasized content: `_Emphasized_`.
    EmphNode: Emph
}

impl EmphNode {
    /// The contents of the emphasis node.
    pub fn body(&self) -> Markup {
        self.0
            .cast_first_child()
            .expect("emphasis node is missing markup body")
    }
}

/// A raw block with optional syntax highlighting: `` `...` ``.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct RawNode {
    /// An optional identifier specifying the language to syntax-highlight in.
    pub lang: Option<EcoString>,
    /// The raw text, determined as the raw string between the backticks trimmed
    /// according to the above rules.
    pub text: EcoString,
    /// Whether the element is block-level, that is, it has 3+ backticks
    /// and contains at least one newline.
    pub block: bool,
}

/// A math formula: `$a^2 + b^2 = c^2$`.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct MathNode {
    /// The formula between the dollars / brackets.
    pub formula: EcoString,
    /// Whether the formula is display-level, that is, it is surrounded by
    /// `$[..]$`.
    pub display: bool,
}

node! {
    /// A section heading: `= Introduction`.
    HeadingNode: Heading
}

impl HeadingNode {
    /// The contents of the heading.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("heading is missing markup body")
    }

    /// The section depth (numer of equals signs).
    pub fn level(&self) -> usize {
        self.0.children().filter(|n| n.kind() == &NodeKind::Eq).count()
    }
}

node! {
    /// An item in an unordered list: `- ...`.
    ListNode: List
}

impl ListNode {
    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("list node is missing body")
    }
}

node! {
    /// An item in an enumeration (ordered list): `1. ...`.
    EnumNode: Enum
}

impl EnumNode {
    /// The contents of the list item.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("enum node is missing body")
    }

    /// The number, if any.
    pub fn number(&self) -> Option<usize> {
        self.0
            .children()
            .find_map(|node| match node.kind() {
                NodeKind::EnumNumbering(num) => Some(*num),
                _ => None,
            })
            .expect("enum node is missing number")
    }
}

/// An expression.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Expr {
    /// A literal: `1`, `true`, ...
    Lit(Lit),
    /// An identifier: `left`.
    Ident(Ident),
    /// A code block: `{ let x = 1; x + 2 }`.
    Code(CodeBlock),
    /// A content block: `[*Hi* there!]`.
    Content(ContentBlock),
    /// A grouped expression: `(1 + 2)`.
    Group(GroupExpr),
    /// An array expression: `(1, "hi", 12cm)`.
    Array(ArrayExpr),
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    Dict(DictExpr),
    /// A unary operation: `-x`.
    Unary(UnaryExpr),
    /// A binary operation: `a + b`.
    Binary(BinaryExpr),
    /// A field access: `properties.age`.
    FieldAccess(FieldAccess),
    /// An invocation of a function: `f(x, y)`.
    FuncCall(FuncCall),
    /// An invocation of a method: `array.push(v)`.
    MethodCall(MethodCall),
    /// A closure expression: `(x, y) => z`.
    Closure(ClosureExpr),
    /// A let expression: `let x = 1`.
    Let(LetExpr),
    /// A set expression: `set text(...)`.
    Set(SetExpr),
    /// A show expression: `show node: heading as [*{nody.body}*]`.
    Show(ShowExpr),
    /// A wrap expression: `wrap body in columns(2, body)`.
    Wrap(WrapExpr),
    /// An if-else expression: `if x { y } else { z }`.
    If(IfExpr),
    /// A while loop expression: `while x { y }`.
    While(WhileExpr),
    /// A for loop expression: `for x in y { z }`.
    For(ForExpr),
    /// An import expression: `import a, b, c from "utils.typ"`.
    Import(ImportExpr),
    /// An include expression: `include "chapter1.typ"`.
    Include(IncludeExpr),
    /// A break expression: `break`.
    Break(BreakExpr),
    /// A continue expression: `continue`.
    Continue(ContinueExpr),
    /// A return expression: `return`.
    Return(ReturnExpr),
}

impl TypedNode for Expr {
    fn from_red(node: RedRef) -> Option<Self> {
        match node.kind() {
            NodeKind::Ident(_) => node.cast().map(Self::Ident),
            NodeKind::CodeBlock => node.cast().map(Self::Code),
            NodeKind::ContentBlock => node.cast().map(Self::Content),
            NodeKind::GroupExpr => node.cast().map(Self::Group),
            NodeKind::ArrayExpr => node.cast().map(Self::Array),
            NodeKind::DictExpr => node.cast().map(Self::Dict),
            NodeKind::UnaryExpr => node.cast().map(Self::Unary),
            NodeKind::BinaryExpr => node.cast().map(Self::Binary),
            NodeKind::FieldAccess => node.cast().map(Self::FieldAccess),
            NodeKind::FuncCall => node.cast().map(Self::FuncCall),
            NodeKind::MethodCall => node.cast().map(Self::MethodCall),
            NodeKind::ClosureExpr => node.cast().map(Self::Closure),
            NodeKind::LetExpr => node.cast().map(Self::Let),
            NodeKind::SetExpr => node.cast().map(Self::Set),
            NodeKind::ShowExpr => node.cast().map(Self::Show),
            NodeKind::WrapExpr => node.cast().map(Self::Wrap),
            NodeKind::IfExpr => node.cast().map(Self::If),
            NodeKind::WhileExpr => node.cast().map(Self::While),
            NodeKind::ForExpr => node.cast().map(Self::For),
            NodeKind::ImportExpr => node.cast().map(Self::Import),
            NodeKind::IncludeExpr => node.cast().map(Self::Include),
            NodeKind::BreakExpr => node.cast().map(Self::Break),
            NodeKind::ContinueExpr => node.cast().map(Self::Continue),
            NodeKind::ReturnExpr => node.cast().map(Self::Return),
            _ => node.cast().map(Self::Lit),
        }
    }

    fn as_red(&self) -> RedRef<'_> {
        match self {
            Self::Lit(v) => v.as_red(),
            Self::Code(v) => v.as_red(),
            Self::Content(v) => v.as_red(),
            Self::Ident(v) => v.as_red(),
            Self::Array(v) => v.as_red(),
            Self::Dict(v) => v.as_red(),
            Self::Group(v) => v.as_red(),
            Self::Unary(v) => v.as_red(),
            Self::Binary(v) => v.as_red(),
            Self::FieldAccess(v) => v.as_red(),
            Self::FuncCall(v) => v.as_red(),
            Self::MethodCall(v) => v.as_red(),
            Self::Closure(v) => v.as_red(),
            Self::Let(v) => v.as_red(),
            Self::Set(v) => v.as_red(),
            Self::Show(v) => v.as_red(),
            Self::Wrap(v) => v.as_red(),
            Self::If(v) => v.as_red(),
            Self::While(v) => v.as_red(),
            Self::For(v) => v.as_red(),
            Self::Import(v) => v.as_red(),
            Self::Include(v) => v.as_red(),
            Self::Break(v) => v.as_red(),
            Self::Continue(v) => v.as_red(),
            Self::Return(v) => v.as_red(),
        }
    }
}

impl Expr {
    /// Whether the expression can be shortened in markup with a hashtag.
    pub fn has_short_form(&self) -> bool {
        matches!(
            self,
            Self::Ident(_)
                | Self::FuncCall(_)
                | Self::Let(_)
                | Self::Set(_)
                | Self::Show(_)
                | Self::Wrap(_)
                | Self::If(_)
                | Self::While(_)
                | Self::For(_)
                | Self::Import(_)
                | Self::Include(_)
        )
    }
}

node! {
    /// A literal: `1`, `true`, ...
    Lit: NodeKind::None
       | NodeKind::Auto
       | NodeKind::Bool(_)
       | NodeKind::Int(_)
       | NodeKind::Float(_)
       | NodeKind::Numeric(_, _)
       | NodeKind::Str(_)
}

impl Lit {
    /// The kind of literal.
    pub fn kind(&self) -> LitKind {
        match *self.0.kind() {
            NodeKind::None => LitKind::None,
            NodeKind::Auto => LitKind::Auto,
            NodeKind::Bool(v) => LitKind::Bool(v),
            NodeKind::Int(v) => LitKind::Int(v),
            NodeKind::Float(v) => LitKind::Float(v),
            NodeKind::Numeric(v, unit) => LitKind::Numeric(v, unit),
            NodeKind::Str(ref v) => LitKind::Str(v.clone()),
            _ => panic!("literal is of wrong kind"),
        }
    }
}

/// The kind of a literal.
#[derive(Debug, Clone, PartialEq)]
pub enum LitKind {
    /// The none literal: `none`.
    None,
    /// The auto literal: `auto`.
    Auto,
    /// A boolean literal: `true`, `false`.
    Bool(bool),
    /// An integer literal: `120`.
    Int(i64),
    /// A floating-point literal: `1.2`, `10e-4`.
    Float(f64),
    /// A numeric literal with a unit: `12pt`, `3cm`, `2em`, `90deg`, `50%`.
    Numeric(f64, Unit),
    /// A string literal: `"hello!"`.
    Str(EcoString),
}

/// Unit of a numeric value.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Unit {
    /// An absolute length unit.
    Length(LengthUnit),
    /// An angular unit.
    Angle(AngleUnit),
    /// Font-relative: `1em` is the same as the font size.
    Em,
    /// Fractions: `fr`.
    Fr,
    /// Percentage: `%`.
    Percent,
}

node! {
    /// A code block: `{ let x = 1; x + 2 }`.
    CodeBlock: CodeBlock
}

impl CodeBlock {
    /// The list of expressions contained in the block.
    pub fn exprs(&self) -> impl Iterator<Item = Expr> + '_ {
        self.0.children().filter_map(RedRef::cast)
    }
}

node! {
    /// A content block: `[*Hi* there!]`.
    ContentBlock: ContentBlock
}

impl ContentBlock {
    /// The contained markup.
    pub fn body(&self) -> Markup {
        self.0.cast_first_child().expect("content is missing body")
    }
}

node! {
    /// A grouped expression: `(1 + 2)`.
    GroupExpr: GroupExpr
}

impl GroupExpr {
    /// The wrapped expression.
    pub fn expr(&self) -> Expr {
        self.0.cast_first_child().expect("group is missing expression")
    }
}

node! {
    /// An array expression: `(1, "hi", 12cm)`.
    ArrayExpr: ArrayExpr
}

impl ArrayExpr {
    /// The array items.
    pub fn items(&self) -> impl Iterator<Item = ArrayItem> + '_ {
        self.0.children().filter_map(RedRef::cast)
    }
}

/// An item in an array expresssion.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ArrayItem {
    /// A simple value: `12`.
    Pos(Expr),
    /// A spreaded value: `..things`.
    Spread(Expr),
}

impl TypedNode for ArrayItem {
    fn from_red(node: RedRef) -> Option<Self> {
        match node.kind() {
            NodeKind::Spread => node.cast_first_child().map(Self::Spread),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn as_red(&self) -> RedRef<'_> {
        match self {
            Self::Pos(v) => v.as_red(),
            Self::Spread(v) => v.as_red(),
        }
    }
}

node! {
    /// A dictionary expression: `(thickness: 3pt, pattern: dashed)`.
    DictExpr: DictExpr
}

impl DictExpr {
    /// The named dictionary items.
    pub fn items(&self) -> impl Iterator<Item = DictItem> + '_ {
        self.0.children().filter_map(RedRef::cast)
    }
}

/// An item in an dictionary expresssion.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum DictItem {
    /// A simple named pair: `12`.
    Named(Named),
    /// A spreaded value: `..things`.
    Spread(Expr),
}

impl TypedNode for DictItem {
    fn from_red(node: RedRef) -> Option<Self> {
        match node.kind() {
            NodeKind::Named => node.cast().map(Self::Named),
            NodeKind::Spread => node.cast_first_child().map(Self::Spread),
            _ => None,
        }
    }

    fn as_red(&self) -> RedRef<'_> {
        match self {
            Self::Named(v) => v.as_red(),
            Self::Spread(v) => v.as_red(),
        }
    }
}

node! {
    /// A pair of a name and an expression: `pattern: dashed`.
    Named
}

impl Named {
    /// The name: `pattern`.
    pub fn name(&self) -> Ident {
        self.0.cast_first_child().expect("named pair is missing name")
    }

    /// The right-hand side of the pair: `dashed`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_child().expect("named pair is missing expression")
    }
}

node! {
    /// A unary operation: `-x`.
    UnaryExpr: UnaryExpr
}

impl UnaryExpr {
    /// The operator: `-`.
    pub fn op(&self) -> UnOp {
        self.0
            .children()
            .find_map(|node| UnOp::from_token(node.kind()))
            .expect("unary expression is missing operator")
    }

    /// The expression to operator on: `x`.
    pub fn expr(&self) -> Expr {
        self.0.cast_last_child().expect("unary expression is missing child")
    }
}

/// A unary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum UnOp {
    /// The plus operator: `+`.
    Pos,
    /// The negation operator: `-`.
    Neg,
    /// The boolean `not`.
    Not,
}

impl UnOp {
    /// Try to convert the token into a unary operation.
    pub fn from_token(token: &NodeKind) -> Option<Self> {
        Some(match token {
            NodeKind::Plus => Self::Pos,
            NodeKind::Minus => Self::Neg,
            NodeKind::Not => Self::Not,
            _ => return None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Pos | Self::Neg => 7,
            Self::Not => 4,
        }
    }

    /// The string representation of this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pos => "+",
            Self::Neg => "-",
            Self::Not => "not",
        }
    }
}

node! {
    /// A binary operation: `a + b`.
    BinaryExpr: BinaryExpr
}

impl BinaryExpr {
    /// The binary operator: `+`.
    pub fn op(&self) -> BinOp {
        let mut not = false;
        self.0
            .children()
            .find_map(|node| match node.kind() {
                NodeKind::Not => {
                    not = true;
                    None
                }
                NodeKind::In if not => Some(BinOp::NotIn),
                _ => BinOp::from_token(node.kind()),
            })
            .expect("binary expression is missing operator")
    }

    /// The left-hand side of the operation: `a`.
    pub fn lhs(&self) -> Expr {
        self.0
            .cast_first_child()
            .expect("binary expression is missing left-hand side")
    }

    /// The right-hand side of the operation: `b`.
    pub fn rhs(&self) -> Expr {
        self.0
            .cast_last_child()
            .expect("binary expression is missing right-hand side")
    }
}

/// A binary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BinOp {
    /// The addition operator: `+`.
    Add,
    /// The subtraction operator: `-`.
    Sub,
    /// The multiplication operator: `*`.
    Mul,
    /// The division operator: `/`.
    Div,
    /// The short-circuiting boolean `and`.
    And,
    /// The short-circuiting boolean `or`.
    Or,
    /// The equality operator: `==`.
    Eq,
    /// The inequality operator: `!=`.
    Neq,
    /// The less-than operator: `<`.
    Lt,
    /// The less-than or equal operator: `<=`.
    Leq,
    /// The greater-than operator: `>`.
    Gt,
    /// The greater-than or equal operator: `>=`.
    Geq,
    /// The assignment operator: `=`.
    Assign,
    /// The containment operator: `in`.
    In,
    /// The inversed containment operator: `not in`.
    NotIn,
    /// The add-assign operator: `+=`.
    AddAssign,
    /// The subtract-assign oeprator: `-=`.
    SubAssign,
    /// The multiply-assign operator: `*=`.
    MulAssign,
    /// The divide-assign operator: `/=`.
    DivAssign,
}

impl BinOp {
    /// Try to convert the token into a binary operation.
    pub fn from_token(token: &NodeKind) -> Option<Self> {
        Some(match token {
            NodeKind::Plus => Self::Add,
            NodeKind::Minus => Self::Sub,
            NodeKind::Star => Self::Mul,
            NodeKind::Slash => Self::Div,
            NodeKind::And => Self::And,
            NodeKind::Or => Self::Or,
            NodeKind::EqEq => Self::Eq,
            NodeKind::ExclEq => Self::Neq,
            NodeKind::Lt => Self::Lt,
            NodeKind::LtEq => Self::Leq,
            NodeKind::Gt => Self::Gt,
            NodeKind::GtEq => Self::Geq,
            NodeKind::Eq => Self::Assign,
            NodeKind::In => Self::In,
            NodeKind::PlusEq => Self::AddAssign,
            NodeKind::HyphEq => Self::SubAssign,
            NodeKind::StarEq => Self::MulAssign,
            NodeKind::SlashEq => Self::DivAssign,
            _ => return None,
        })
    }

    /// The precedence of this operator.
    pub fn precedence(self) -> usize {
        match self {
            Self::Mul => 6,
            Self::Div => 6,
            Self::Add => 5,
            Self::Sub => 5,
            Self::Eq => 4,
            Self::Neq => 4,
            Self::Lt => 4,
            Self::Leq => 4,
            Self::Gt => 4,
            Self::Geq => 4,
            Self::In => 4,
            Self::NotIn => 4,
            Self::And => 3,
            Self::Or => 2,
            Self::Assign => 1,
            Self::AddAssign => 1,
            Self::SubAssign => 1,
            Self::MulAssign => 1,
            Self::DivAssign => 1,
        }
    }

    /// The associativity of this operator.
    pub fn associativity(self) -> Associativity {
        match self {
            Self::Add => Associativity::Left,
            Self::Sub => Associativity::Left,
            Self::Mul => Associativity::Left,
            Self::Div => Associativity::Left,
            Self::And => Associativity::Left,
            Self::Or => Associativity::Left,
            Self::Eq => Associativity::Left,
            Self::Neq => Associativity::Left,
            Self::Lt => Associativity::Left,
            Self::Leq => Associativity::Left,
            Self::Gt => Associativity::Left,
            Self::Geq => Associativity::Left,
            Self::In => Associativity::Left,
            Self::NotIn => Associativity::Left,
            Self::Assign => Associativity::Right,
            Self::AddAssign => Associativity::Right,
            Self::SubAssign => Associativity::Right,
            Self::MulAssign => Associativity::Right,
            Self::DivAssign => Associativity::Right,
        }
    }

    /// The string representation of this operation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::And => "and",
            Self::Or => "or",
            Self::Eq => "==",
            Self::Neq => "!=",
            Self::Lt => "<",
            Self::Leq => "<=",
            Self::Gt => ">",
            Self::Geq => ">=",
            Self::In => "in",
            Self::NotIn => "not in",
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
        }
    }
}

/// The associativity of a binary operator.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Associativity {
    /// Left-associative: `a + b + c` is equivalent to `(a + b) + c`.
    Left,
    /// Right-associative: `a = b = c` is equivalent to `a = (b = c)`.
    Right,
}

node! {
    /// A field access: `properties.age`.
    FieldAccess: FieldAccess
}

impl FieldAccess {
    /// The object with the field.
    pub fn object(&self) -> Expr {
        self.0.cast_first_child().expect("field access is missing object")
    }

    /// The name of the field.
    pub fn field(&self) -> Ident {
        self.0.cast_last_child().expect("field access call is missing name")
    }
}

node! {
    /// An invocation of a function: `f(x, y)`.
    FuncCall: FuncCall
}

impl FuncCall {
    /// The function to call.
    pub fn callee(&self) -> Expr {
        self.0.cast_first_child().expect("function call is missing callee")
    }

    /// The arguments to the function.
    pub fn args(&self) -> CallArgs {
        self.0
            .cast_last_child()
            .expect("function call is missing argument list")
    }
}

node! {
    /// An invocation of a method: `array.push(v)`.
    MethodCall: MethodCall
}

impl MethodCall {
    /// The value to call the method on.
    pub fn receiver(&self) -> Expr {
        self.0.cast_first_child().expect("method call is missing callee")
    }

    /// The name of the method.
    pub fn method(&self) -> Ident {
        self.0.cast_last_child().expect("method call is missing name")
    }

    /// The arguments to the method.
    pub fn args(&self) -> CallArgs {
        self.0
            .cast_last_child()
            .expect("method call is missing argument list")
    }
}

node! {
    /// The arguments to a function: `12, draw: false`.
    CallArgs
}

impl CallArgs {
    /// The positional and named arguments.
    pub fn items(&self) -> impl Iterator<Item = CallArg> + '_ {
        self.0.children().filter_map(RedRef::cast)
    }
}

/// An argument to a function call.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CallArg {
    /// A positional argument: `12`.
    Pos(Expr),
    /// A named argument: `draw: false`.
    Named(Named),
    /// A spreaded argument: `..things`.
    Spread(Expr),
}

impl TypedNode for CallArg {
    fn from_red(node: RedRef) -> Option<Self> {
        match node.kind() {
            NodeKind::Named => node.cast().map(Self::Named),
            NodeKind::Spread => node.cast_first_child().map(Self::Spread),
            _ => node.cast().map(Self::Pos),
        }
    }

    fn as_red(&self) -> RedRef<'_> {
        match self {
            Self::Pos(v) => v.as_red(),
            Self::Named(v) => v.as_red(),
            Self::Spread(v) => v.as_red(),
        }
    }
}

node! {
    /// A closure expression: `(x, y) => z`.
    ClosureExpr: ClosureExpr
}

impl ClosureExpr {
    /// The name of the closure.
    ///
    /// This only exists if you use the function syntax sugar: `let f(x) = y`.
    pub fn name(&self) -> Option<Ident> {
        self.0.cast_first_child()
    }

    /// The parameter bindings.
    pub fn params(&self) -> impl Iterator<Item = ClosureParam> + '_ {
        self.0
            .children()
            .find(|x| x.kind() == &NodeKind::ClosureParams)
            .expect("closure is missing parameter list")
            .children()
            .filter_map(RedRef::cast)
    }

    /// The body of the closure.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("closure is missing body")
    }
}

/// A parameter to a closure.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum ClosureParam {
    /// A positional parameter: `x`.
    Pos(Ident),
    /// A named parameter with a default value: `draw: false`.
    Named(Named),
    /// A parameter sink: `..args`.
    Sink(Ident),
}

impl TypedNode for ClosureParam {
    fn from_red(node: RedRef) -> Option<Self> {
        match node.kind() {
            NodeKind::Ident(_) => node.cast().map(Self::Pos),
            NodeKind::Named => node.cast().map(Self::Named),
            NodeKind::Spread => node.cast_first_child().map(Self::Sink),
            _ => None,
        }
    }

    fn as_red(&self) -> RedRef<'_> {
        match self {
            Self::Pos(v) => v.as_red(),
            Self::Named(v) => v.as_red(),
            Self::Sink(v) => v.as_red(),
        }
    }
}

node! {
    /// A let expression: `let x = 1`.
    LetExpr
}

impl LetExpr {
    /// The binding to assign to.
    pub fn binding(&self) -> Ident {
        match self.0.cast_first_child() {
            Some(Expr::Ident(binding)) => binding,
            Some(Expr::Closure(closure)) => {
                closure.name().expect("let-bound closure is missing name")
            }
            _ => panic!("let expression is missing binding"),
        }
    }

    /// The expression the binding is initialized with.
    pub fn init(&self) -> Option<Expr> {
        if self.0.cast_first_child::<Ident>().is_some() {
            self.0.children().filter_map(RedRef::cast).nth(1)
        } else {
            // This is a let .. with expression.
            self.0.cast_first_child()
        }
    }
}

node! {
    /// A set expression: `set text(...)`.
    SetExpr
}

impl SetExpr {
    /// The function to set style properties for.
    pub fn target(&self) -> Ident {
        self.0.cast_first_child().expect("set rule is missing target")
    }

    /// The style properties to set.
    pub fn args(&self) -> CallArgs {
        self.0.cast_last_child().expect("set rule is missing argument list")
    }
}

node! {
    /// A show expression: `show node: heading as [*{nody.body}*]`.
    ShowExpr
}

impl ShowExpr {
    /// The binding to assign to.
    pub fn binding(&self) -> Ident {
        self.0.cast_first_child().expect("show rule is missing binding")
    }

    /// The function to customize with this show rule.
    pub fn target(&self) -> Ident {
        self.0
            .children()
            .filter_map(RedRef::cast)
            .nth(1)
            .expect("show rule is missing target")
    }

    /// The expression that realizes the node.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("show rule is missing body")
    }
}

node! {
    /// A wrap expression: wrap body in columns(2, body)`.
    WrapExpr
}

impl WrapExpr {
    /// The binding to assign the remaining markup to.
    pub fn binding(&self) -> Ident {
        self.0.cast_first_child().expect("wrap expression is missing binding")
    }

    /// The expression to evaluate.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("wrap expression is missing body")
    }
}

node! {
    /// An if-else expression: `if x { y } else { z }`.
    IfExpr
}

impl IfExpr {
    /// The condition which selects the body to evaluate.
    pub fn condition(&self) -> Expr {
        self.0.cast_first_child().expect("if expression is missing condition")
    }

    /// The expression to evaluate if the condition is true.
    pub fn if_body(&self) -> Expr {
        self.0
            .children()
            .filter_map(RedRef::cast)
            .nth(1)
            .expect("if expression is missing body")
    }

    /// The expression to evaluate if the condition is false.
    pub fn else_body(&self) -> Option<Expr> {
        self.0.children().filter_map(RedRef::cast).nth(2)
    }
}

node! {
    /// A while loop expression: `while x { y }`.
    WhileExpr
}

impl WhileExpr {
    /// The condition which selects whether to evaluate the body.
    pub fn condition(&self) -> Expr {
        self.0.cast_first_child().expect("while loop is missing condition")
    }

    /// The expression to evaluate while the condition is true.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("while loop is missing body")
    }
}

node! {
    /// A for loop expression: `for x in y { z }`.
    ForExpr
}

impl ForExpr {
    /// The pattern to assign to.
    pub fn pattern(&self) -> ForPattern {
        self.0.cast_first_child().expect("for loop is missing pattern")
    }

    /// The expression to iterate over.
    pub fn iter(&self) -> Expr {
        self.0.cast_first_child().expect("for loop is missing iterable")
    }

    /// The expression to evaluate for each iteration.
    pub fn body(&self) -> Expr {
        self.0.cast_last_child().expect("for loop is missing body")
    }
}

node! {
    /// A for-in loop expression: `for x in y { z }`.
    ForPattern
}

impl ForPattern {
    /// The key part of the pattern: index for arrays, name for dictionaries.
    pub fn key(&self) -> Option<Ident> {
        let mut children = self.0.children().filter_map(RedRef::cast);
        let key = children.next();
        if children.next().is_some() { key } else { None }
    }

    /// The value part of the pattern.
    pub fn value(&self) -> Ident {
        self.0.cast_last_child().expect("for loop pattern is missing value")
    }
}

node! {
    /// An import expression: `import a, b, c from "utils.typ"`.
    ImportExpr
}

impl ImportExpr {
    /// The items to be imported.
    pub fn imports(&self) -> Imports {
        self.0
            .children()
            .find_map(|node| match node.kind() {
                NodeKind::Star => Some(Imports::Wildcard),
                NodeKind::ImportItems => {
                    let items = node.children().filter_map(RedRef::cast).collect();
                    Some(Imports::Items(items))
                }
                _ => None,
            })
            .expect("import is missing items")
    }

    /// The location of the importable file.
    pub fn path(&self) -> Expr {
        self.0.cast_last_child().expect("import is missing path")
    }
}

/// The items that ought to be imported from a file.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Imports {
    /// All items in the scope of the file should be imported.
    Wildcard,
    /// The specified items from the file should be imported.
    Items(Vec<Ident>),
}

node! {
    /// An include expression: `include "chapter1.typ"`.
    IncludeExpr
}

impl IncludeExpr {
    /// The location of the file to be included.
    pub fn path(&self) -> Expr {
        self.0.cast_last_child().expect("include is missing path")
    }
}

node! {
    /// A break expression: `break`.
    BreakExpr
}

node! {
    /// A continue expression: `continue`.
    ContinueExpr
}

node! {
    /// A return expression: `return x + 1`.
    ReturnExpr
}

impl ReturnExpr {
    /// The expression to return.
    pub fn body(&self) -> Option<Expr> {
        self.0.cast_last_child()
    }
}

node! {
    /// An identifier.
    Ident: NodeKind::Ident(_)
}

impl Ident {
    /// Take out the contained [`EcoString`].
    pub fn take(self) -> EcoString {
        match self.0.green {
            Green::Token(GreenData { kind: NodeKind::Ident(id), .. }) => id,
            _ => panic!("identifier is of wrong kind"),
        }
    }
}

impl Deref for Ident {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match &self.0.green {
            Green::Token(GreenData { kind: NodeKind::Ident(id), .. }) => id,
            _ => panic!("identifier is of wrong kind"),
        }
    }
}

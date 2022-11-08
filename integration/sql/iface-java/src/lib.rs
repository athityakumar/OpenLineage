extern crate jni;
extern crate openlineage_sql as rust_impl;

use anyhow::Result;
use jni::errors::Error;
use jni::objects::{JClass, JList, JObject, JString, JValue};
use jni::sys::{jobject, jstring};
use jni::JNIEnv;

use rust_impl::{get_generic_dialect, parse_multiple_statements};

trait AsJavaObject {
    fn as_java_object<'a, 'b>(&'b self, env: &'a JNIEnv) -> Result<JObject<'a>> {
        let classname = Self::java_class_name();
        let java_class = env.find_class(classname)?;
        let signature = Self::ctor_signature();
        let args = self.ctor_arguments(env)?;

        let obj = env.new_object(java_class, signature, &args)?;
        Ok(obj)
    }

    fn java_class_name() -> &'static str;
    fn ctor_signature() -> &'static str;
    fn ctor_arguments<'a, 'b>(&'b self, env: &'a JNIEnv) -> Result<Box<[JValue<'a>]>>;
}

impl AsJavaObject for rust_impl::SqlMeta {
    fn java_class_name() -> &'static str {
        "io/openlineage/sql/SqlMeta"
    }

    fn ctor_signature() -> &'static str {
        "(Ljava/util/List;Ljava/util/List;)V"
    }

    fn ctor_arguments<'a, 'b>(&'b self, env: &'a JNIEnv) -> Result<Box<[JValue<'a>]>> {
        let array_list_class = env
            .find_class("java/util/ArrayList")
            .expect("Couldn't find the ArrayList class");
        let ins = env
            .new_object(array_list_class, "()V", &[])
            .expect("Coudln't create a new ArrayList");
        let outs = env
            .new_object(array_list_class, "()V", &[])
            .expect("Coudln't create a new ArrayList");
        let ins = JList::from_env(env, ins).unwrap();
        let outs = JList::from_env(env, outs).unwrap();

        for e in &self.in_tables {
            ins.add(e.as_java_object(env)?)?;
        }
        for e in &self.out_tables {
            outs.add(e.as_java_object(env)?)?;
        }

        Ok(Box::new([
            JValue::Object(ins.into()),
            JValue::Object(outs.into()),
        ]))
    }
}

impl AsJavaObject for rust_impl::DbTableMeta {
    fn java_class_name() -> &'static str {
        "io/openlineage/sql/DbTableMeta"
    }

    fn ctor_signature() -> &'static str {
        "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"
    }

    fn ctor_arguments<'a, 'b>(&'b self, env: &'a JNIEnv) -> Result<Box<[JValue<'a>]>> {
        let arg1 = match &self.database {
            Some(d) => env.new_string(d)?.into(),
            None => JObject::null(),
        };
        let arg2 = match &self.schema {
            Some(s) => env.new_string(s)?.into(),
            None => JObject::null(),
        };
        let arg3 = env.new_string(&self.name)?.into();

        Ok(Box::new([
            JValue::Object(arg1),
            JValue::Object(arg2),
            JValue::Object(arg3),
        ]))
    }
}

#[no_mangle]
pub extern "system" fn Java_io_openlineage_sql_OpenLineageSql_parse(
    env: JNIEnv,
    _class: JClass,
    sql: JObject,
    dialect: JString,
    default_schema: JString,
) -> jobject {
    let f = || -> Result<jobject> {
        let sql = env.get_list(sql)?;
        let mut vec_sql: Vec<String> = vec![];
        let size = sql.size()?;
        for i in 0..size {
            let item = sql.get(i)?;
            if let Some(i) = item {
                let s: String = env.get_string(i.into())?.into();
                vec_sql.push(s);
            }
        }

        let dialect: Option<String> = match env.get_string(dialect) {
            Err(Error::NullPtr(_)) => None,
            s => Some(s?.into()),
        };
        let dialect = get_generic_dialect(dialect.as_deref());

        let default_schema: Option<String> = match env.get_string(default_schema) {
            Err(Error::NullPtr(_)) => None,
            s => Some(s?.into()),
        };

        let parsed = parse_multiple_statements(
            vec_sql.iter().map(String::as_str).collect(),
            dialect,
            default_schema,
        )?;
        Ok(parsed.as_java_object(&env)?.into_inner())
    };

    match f() {
        Ok(obj) => obj,
        Err(err) => {
            env.throw_new("java/lang/RuntimeException", err.to_string())
                .unwrap();
            JObject::null().into_inner()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_io_openlineage_sql_OpenLineageSql_provider(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let output = env.new_string("rust").unwrap();
    output.into_inner()
}